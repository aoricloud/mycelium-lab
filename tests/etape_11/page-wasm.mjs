// Fait tourner la vraie page (`docs/index.html` + `docs/app.js`) dans un faux navigateur, avec le vrai module, et joue la
// case « Densité ».
//
// Usage : node page-wasm.mjs <dossier docs> <fichier.wasm> <dossier de travail>
//
// C'est `tests/etape_8/page-wasm.mjs` (le faux navigateur : un faux `document` dont les éléments sont ceux que `index.html`
// déclare, un faux contexte 2D qui note ses appels, une fausse adresse, un faux `fetch` et un faux `requestAnimationFrame`
// que le script fait avancer image par image) avec, en plus : la case à cocher (un clic la coche et lance les événements
// `click`, `input` puis `change`, dans l'ordre d'un vrai navigateur), le lissage noté dans chaque `drawImage`, les canvas
// créés par la page, et le compte des appels au module. Les jeux de l'étape 8 (adresse, menu, boutons, message d'arrêt)
// ne sont pas rejoués : `etape_8_artefact.rs` les joue toujours, page décochée.
//
// Ce que le script joue, pour les six scénarios : la page s'ouvre décochée (la légende cachée, et la page ne lit pas
// la densité) ; cochée en pause au pas de référence (la couche de la page, une case par pixel, est exactement la carte de
// densité du module sur la rampe de `densite.png` : des cases nettes, sans hyphe ni vignettage, les obstacles dessinés
// comme d'habitude par-dessus, la bordure intacte, au plus `largeur × hauteur` lectures et un maximum par image) ; décochée
// de nouveau (exactement le dessin d'avant) ; cochée à l'ouverture, au temps 0 (la carte vide), en marche (une image
// de la carte à chaque image, sur la colonie qui avance), en pause, au changement de scénario, à « Relancer », à une
// adresse changée et à la fin de la croissance. À chaque fois, les légendes disent ce qui est dessiné : cochée, celle de
// la densité et non celles du substrat et de la tache ; décochée, l'inverse (la tache seulement si le scénario en a une).
//
// Il écrit une ligne `couche=<scénario>,<graine>,<pas>,<hache>` pour chaque couche de densité jouée au temps 0 et au pas de
// référence (`hache` : le SHA-256 de ses octets RGBA), que `etape_11_artefact.rs` recalcule avec `piece::carte_de_densite`,
// puis `ok <n>` (n : le nombre de vérifications faites) s'il n'a rien trouvé à redire ; au premier écart, il écrit un
// message sur la sortie d'erreur et sort avec le code 1.
//
// Aucune dépendance : les modules intégrés de Node seulement.

import { createHash } from "node:crypto";
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

// Les six scénarios, d'après la spécification : numéro, nom dans l'adresse, boîte, pas de référence, obstacles, et si
// le substrat a une tache de nutriments.
const SCENARIOS = [
  { numero: 0, nom: "libre", boite: 161, duree: 80, obstacles: 0, tache: false },
  { numero: 1, nom: "terrain", boite: 100, duree: 80, obstacles: 2, tache: true },
  { numero: 2, nom: "tache", boite: 100, duree: 100, obstacles: 0, tache: true },
  { numero: 3, nom: "tache-sans", boite: 100, duree: 100, obstacles: 0, tache: true },
  { numero: 4, nom: "mur", boite: 100, duree: 120, obstacles: 1, tache: false },
  { numero: 5, nom: "mur-sans", boite: 100, duree: 120, obstacles: 1, tache: false },
];
const TAILLE_CANVAS = 600;
// Les couleurs de `densite.png` (examples/mesure_piece.rs), écrites ici d'après sa doc : le fond d'une case sans hyphe,
// et les deux bouts de la rampe.
const FOND = [16, 16, 24];
const RAMPE_SOMBRE = [70, 20, 110];
const RAMPE_CLAIRE = [255, 240, 90];
// Les couleurs des obstacles de la page, que la case « Densité » ne change pas.
const COULEUR_OBSTACLE = "#5b6b73";

let verifications = 0;

/** Compte une vérification et échoue, avec ce message, si elle est fausse. */
function verifier(condition, message) {
  verifications += 1;
  if (!condition) {
    throw new Error(message);
  }
}

function egal(obtenu, attendu, message) {
  verifier(obtenu === attendu, `${message} : obtenu ${JSON.stringify(obtenu)}, attendu ${JSON.stringify(attendu)}`);
}

// ---------------------------------------------------------------------------------------------------------------
// Les éléments de la page, tels que `index.html` les déclare.
// ---------------------------------------------------------------------------------------------------------------

/** Les attributs d'une balise ouvrante : `nom="valeur"`, ou `nom` seul. */
function attributs(texte) {
  const resultat = {};
  for (const m of texte.matchAll(/([a-zA-Z][a-zA-Z0-9:-]*)(?:="([^"]*)")?/g)) {
    resultat[m[1]] = m[2] === undefined ? "" : m[2];
  }
  return resultat;
}

/** Le texte d'un élément simple : ce qui sépare sa balise ouvrante de sa balise fermante, sans les balises. */
function texteInterne(html, debut, nom) {
  const fin = html.indexOf(`</${nom}>`, debut);
  return fin === -1 ? "" : html.slice(debut, fin).replace(/<[^>]*>/g, "").replace(/\s+/g, " ").trim();
}

/** Un faux contexte 2D : il ne dessine rien et note chaque appel dans `journalCourant`. */
function fauxContexte() {
  const proprietes = {};
  const contexte = new Proxy(
    {},
    {
      get(_cible, nom) {
        if (nom in proprietes) {
          return proprietes[nom];
        }
        if (nom === "createImageData") {
          return (largeur, hauteur) => ({ width: largeur, height: hauteur, data: new Uint8ClampedArray(4 * largeur * hauteur) });
        }
        if (nom === "createRadialGradient" || nom === "createLinearGradient") {
          return () => ({ addColorStop() {} });
        }
        return (...args) => {
          const entree = { appel: nom, args };
          if (nom === "fillRect" || nom === "strokeRect") {
            entree.style = nom === "fillRect" ? proprietes.fillStyle : proprietes.strokeStyle;
          }
          if (nom === "drawImage") {
            entree.lissage = proprietes.imageSmoothingEnabled;
          }
          contexte.journalCourant.push(entree);
        };
      },
      set(_cible, nom, valeur) {
        proprietes[nom] = valeur;
        return true;
      },
    },
  );
  contexte.journalCourant = [];
  return contexte;
}

class Element {
  constructor(tag, attrs, texte, options) {
    this.tagName = tag.toUpperCase();
    this.attrs = { ...attrs };
    this.id = attrs.id ?? "";
    this.textContent = texte;
    this.ecouteurs = {};
    this.disabled = "disabled" in attrs;
    this.hidden = "hidden" in attrs;
    this.checked = "checked" in attrs;
    this.width = 300;
    this.height = 150;
    this.options = options ?? null;
    this.contexte = null;
    this.valeur = attrs.value ?? "";
    if (options) {
      const choisie = options.find((o) => o.selected) ?? options[0];
      this.valeur = choisie ? choisie.valeur : "";
    }
  }

  // Dans un menu, donner une valeur qui n'est celle d'aucune option vide le choix, comme dans un navigateur.
  get value() {
    return this.valeur;
  }

  set value(nouvelle) {
    const texte = String(nouvelle);
    this.valeur = this.options && !this.options.some((o) => o.valeur === texte) ? "" : texte;
  }

  // La page n'écrit que du texte (`textContent`) : toute écriture de HTML est une faute, le faux navigateur la refuse.
  get innerHTML() {
    return this.textContent;
  }

  set innerHTML(_valeur) {
    throw new Error(`la page écrit du HTML dans l'élément « ${this.id} » (innerHTML)`);
  }

  set outerHTML(_valeur) {
    throw new Error(`la page écrit du HTML à la place de l'élément « ${this.id} » (outerHTML)`);
  }

  insertAdjacentHTML() {
    throw new Error(`la page écrit du HTML près de l'élément « ${this.id} » (insertAdjacentHTML)`);
  }

  addEventListener(type, fonction) {
    (this.ecouteurs[type] ??= []).push(fonction);
  }

  dispatch(type, init = {}) {
    for (const fonction of this.ecouteurs[type] ?? []) {
      fonction({ type, target: this, preventDefault() {}, ...init });
    }
  }

  // Un clic sur une case à cocher la coche ou la décoche d'abord, puis lance `click`, `input` et `change`, dans l'ordre
  // d'un vrai navigateur.
  click() {
    if (this.disabled) {
      return;
    }
    const caseACocher = this.attrs.type === "checkbox";
    if (caseACocher) {
      this.checked = !this.checked;
    }
    this.dispatch("click");
    if (caseACocher) {
      this.dispatch("input");
      this.dispatch("change");
    }
  }

  setAttribute(nom, valeur) {
    this.attrs[nom] = String(valeur);
  }

  getAttribute(nom) {
    return nom in this.attrs ? this.attrs[nom] : null;
  }

  getBoundingClientRect() {
    return { left: 0, top: 0, right: TAILLE_CANVAS, bottom: TAILLE_CANVAS, width: TAILLE_CANVAS, height: TAILLE_CANVAS };
  }

  getContext(sorte) {
    if (sorte !== "2d") {
      return null;
    }
    this.contexte ??= fauxContexte();
    return this.contexte;
  }
}

/** Les éléments que déclare `index.html`, par identifiant. */
function elementsDeLaPage(html) {
  const elements = new Map();
  for (const m of html.matchAll(/<([a-zA-Z][a-zA-Z0-9]*)\b([^>]*)>/g)) {
    const attrs = attributs(m[2]);
    if (!("id" in attrs)) {
      continue;
    }
    let options = null;
    if (m[1].toLowerCase() === "select") {
      const fin = html.indexOf("</select>", m.index);
      const interieur = html.slice(m.index, fin);
      options = [...interieur.matchAll(/<option\b([^>]*)>/g)].map((o) => {
        const a = attributs(o[1]);
        return { valeur: a.value ?? "", selected: "selected" in a };
      });
    }
    const texte = options ? "" : texteInterne(html, m.index + m[0].length, m[1]);
    elements.set(attrs.id, new Element(m[1], attrs, texte, options));
  }
  return elements;
}

// ---------------------------------------------------------------------------------------------------------------
// Le faux navigateur.
// ---------------------------------------------------------------------------------------------------------------

const [dossierDocs, cheminModule, dossierTravail] = process.argv.slice(2);
if (!dossierDocs || !cheminModule || !dossierTravail || process.argv.length !== 5) {
  process.stderr.write("usage : node page-wasm.mjs <dossier docs> <fichier.wasm> <dossier de travail>\n");
  process.exit(1);
}

const html = readFileSync(join(dossierDocs, "index.html"), "utf8");
const octetsDuModule = readFileSync(cheminModule);
const instancierReel = WebAssembly.instantiate.bind(WebAssembly);
let numeroDeLancement = 0;

/**
 * Ouvre la page avec cette adresse (`hash`) : rend de quoi la piloter et la lire. Avec `cochee`, la case « Densité » est
 * déjà cochée quand `app.js` s'exécute (un navigateur restitue l'état des cases au rechargement).
 */
async function ouvrir(hash, dossierPage, { cochee = false } = {}) {
  numeroDeLancement += 1;
  const elements = elementsDeLaPage(html);
  if (cochee) {
    elements.get("densite").checked = true;
  }
  const page = {
    elements,
    // Combien de fois la page a appelé chaque fonction du module : on remet `compteurs` à `{}` pour compter une action.
    compteurs: {},
    exportations: null,
    imageSuivante: null,
    ecouteursFenetre: {},
    hash,
  };

  page.el = (id) => {
    const element = elements.get(id);
    verifier(element !== undefined, `index.html n'a pas d'élément d'id « ${id} »`);
    return element;
  };

  globalThis.window = globalThis;
  globalThis.location = {
    get hash() {
      return page.hash;
    },
  };
  globalThis.history = {
    replaceState(_etat, _titre, url) {
      if (typeof url === "string" && url.startsWith("#")) {
        page.hash = url;
      }
    },
  };
  globalThis.devicePixelRatio = 1;
  globalThis.addEventListener = (type, fonction) => {
    (page.ecouteursFenetre[type] ??= []).push(fonction);
  };
  globalThis.document = {
    getElementById: (id) => elements.get(id) ?? null,
    createElement: (tag) => new Element(tag, {}, "", null),
  };
  globalThis.requestAnimationFrame = (fonction) => {
    page.imageSuivante = fonction;
    return 1;
  };
  globalThis.ResizeObserver = class {
    observe() {}
  };
  globalThis.fetch = async () => ({ ok: true, status: 200, arrayBuffer: async () => octetsDuModule });
  WebAssembly.instantiate = async (octets, importations) => {
    const resultat = await instancierReel(octets, importations);
    page.exportations = resultat.instance.exports;
    // Les exportations d'un module sont figées : on rend un objet ordinaire, dont chaque fonction compte ses appels.
    const enveloppe = {};
    for (const [nom, valeur] of Object.entries(resultat.instance.exports)) {
      enveloppe[nom] =
        typeof valeur === "function"
          ? (...args) => {
              page.compteurs[nom] = (page.compteurs[nom] ?? 0) + 1;
              return valeur(...args);
            }
          : valeur;
    }
    return { ...resultat, instance: { exports: enveloppe } };
  };

  // Chaque ouverture importe sa propre copie de `app.js` : un module ne s'exécute qu'une fois, et la page, elle,
  // s'exécute à chaque ouverture.
  const copie = join(dossierPage, `app-${numeroDeLancement}.js`);
  copyFileSync(join(dossierDocs, "app.js"), copie);
  await import(pathToFileURL(copie).href);

  // Le module se charge de façon asynchrone : on laisse tourner, au plus vingt secondes, jusqu'à ce que la page ait
  // démarré une colonie.
  const limite = Date.now() + 20000;
  while (Date.now() < limite && !(page.compteurs.mycelium_demarrer > 0)) {
    await new Promise((resolve) => setTimeout(resolve, 2));
  }
  verifier(
    page.compteurs.mycelium_demarrer > 0,
    `la page n'a démarré aucune colonie (adresse « ${hash} », état « ${elements.get("etat")?.textContent} », message « ${elements.get("message")?.textContent} »)`,
  );

  /** Fait avancer la page de `n` images d'animation. */
  page.images = (n) => {
    for (let i = 0; i < n; i++) {
      const fonction = page.imageSuivante;
      page.imageSuivante = null;
      verifier(fonction !== null, "la page n'a pas demandé d'image suivante");
      fonction(0);
    }
  };
  /** Change le menu de scénario, comme le ferait une personne. */
  page.choisirDansLeMenu = (nom) => {
    const menu = page.el("scenario");
    menu.value = nom;
    verifier(menu.value === nom, `le menu n'a pas d'option « ${nom} »`);
    menu.dispatch("change");
  };
  page.cliquer = (id) => page.el(id).click();
  /** Le journal des appels au contexte 2D du canvas ; on le vide en lui donnant `[]`. */
  page.canvas = () => page.el("boite").getContext("2d");
  page.u32 = (nom, ...args) => page.exportations[nom](...args) >>> 0;
  return page;
}

// ---------------------------------------------------------------------------------------------------------------
// Ce que la page a dessiné.
// ---------------------------------------------------------------------------------------------------------------

/** Une entrée du journal, réduite à ce qui se compare : son nom, ses arguments (un objet s'écrit « <objet> »), son style. */
function forme(entree) {
  const args = entree.args.map((a) => (typeof a === "object" && a !== null ? "<objet>" : a));
  return JSON.stringify([entree.appel, ...args, entree.style ?? null, entree.lissage ?? null]);
}

/** Les octets RGBA de la couche hors écran (un pixel par case) que la page a posée sur le canvas à sa dernière image. */
function pixelsDeLaCouche(page) {
  const dessin = page.canvas().journalCourant.findLast((e) => e.appel === "drawImage" && e.args.length === 5);
  verifier(dessin !== undefined, "la page n'a posé aucune image sur le canvas");
  const envois = dessin.args[0].getContext("2d").journalCourant.filter((e) => e.appel === "putImageData");
  verifier(envois.length > 0, "la couche posée sur le canvas n'a jamais reçu de pixels");
  const [image, x, y] = envois.at(-1).args;
  verifier(x === 0 && y === 0, `les pixels de la couche sont posés en (${x}, ${y}) et non en (0, 0)`);
  return Uint8ClampedArray.from(image.data);
}

/** Les rectangles pleins que la page a dessinés sur le canvas, hors les aplats qui couvrent la boîte (le vignettage). */
function rectanglesDObstacles(page) {
  return page
    .canvas()
    .journalCourant.filter((e) => e.appel === "fillRect" && e.args[2] < page.el("boite").width / 2)
    .map((e) => ({ args: e.args, style: e.style }));
}

/** Le dessin que la page vient de faire (le journal du canvas doit avoir été vidé juste avant). */
function dessin(page) {
  const journal = page.canvas().journalCourant;
  return { entrees: journal, formes: journal.map(forme), pixels: pixelsDeLaCouche(page), obstacles: rectanglesDObstacles(page) };
}

/** Ce que la page a posé entre `clip` (le début du dessin de la boîte) et `restore`, et ce qu'elle a dessiné après. */
function decouper(entrees) {
  const debut = entrees.findIndex((e) => e.appel === "clip");
  const fin = entrees.findIndex((e) => e.appel === "restore");
  verifier(debut !== -1 && fin > debut, "le dessin n'a ni `clip` ni `restore` dans l'ordre");
  return { dans: entrees.slice(debut + 1, fin), apres: entrees.slice(fin + 1) };
}

/** La première case (indice, ligne par ligne) où deux images RGBA diffèrent, ou -1 si elles sont égales. */
function premierEcart(a, b) {
  if (a.length !== b.length) {
    return 0;
  }
  for (let i = 0; i < a.length; i++) {
    if (a[i] !== b[i]) {
      return Math.floor(i / 4);
    }
  }
  return -1;
}

function hache(pixels) {
  return createHash("sha256").update(pixels).digest("hex");
}

/**
 * L'image que doit avoir la couche de densité d'après la carte du module : une case sans hyphe (densité nulle) a la
 * couleur du fond ; une case occupée suit la rampe, `niveau = ⌊√(min(d / max, 1)) × 255⌋` puis, canal par canal,
 * `sombre + (clair − sombre) × niveau / 255` avec une division qui arrondit vers zéro (celle de Rust : le canal bleu
 * descend de 110 à 90 et ne doit pas arrondir vers le bas).
 */
function imageDeDensite(m) {
  const largeur = m.mycelium_largeur() >>> 0;
  const hauteur = m.mycelium_hauteur() >>> 0;
  const maximum = m.mycelium_densite_max();
  const image = new Uint8ClampedArray(4 * largeur * hauteur);
  for (let i = 0; i < largeur * hauteur; i++) {
    const densite = m.mycelium_densite(i % largeur, Math.floor(i / largeur));
    const niveau = densite > 0 ? Math.trunc(Math.sqrt(Math.min(densite / maximum, 1)) * 255) : -1;
    for (let canal = 0; canal < 3; canal++) {
      image[4 * i + canal] =
        niveau < 0
          ? FOND[canal]
          : RAMPE_SOMBRE[canal] + Math.trunc(((RAMPE_CLAIRE[canal] - RAMPE_SOMBRE[canal]) * niveau) / 255);
    }
    image[4 * i + 3] = 255;
  }
  return image;
}

/** Les obstacles du milieu, des rectangles de cases `[x0, y0, x1, y1]` dont les deux bords sont inclus. */
function obstaclesDuModule(m) {
  const obstacles = [];
  for (let i = 0; i < (m.mycelium_nb_obstacles() >>> 0); i++) {
    obstacles.push([0, 1, 2, 3].map((k) => m.mycelium_obstacle(i, k)));
  }
  return obstacles;
}

const lumiere = (pixels, i) => pixels[4 * i] + pixels[4 * i + 1] + pixels[4 * i + 2];

/** Vérifie que la page, cochée, vient de dessiner exactement la carte de densité du module ; rend ce dessin. */
function verifierLaDensite(page, contexte) {
  const m = page.exportations;
  const image = dessin(page);
  const ecart = premierEcart(image.pixels, imageDeDensite(m));
  egal(ecart, -1, `${contexte} : la couche de densité diffère de la carte du module (première case, ligne par ligne)`);
  return image;
}

/**
 * Vérifie que les légendes disent ce que la page dessine. Avec la densité (`enDensite`) : celle de la densité, et non
 * celles du substrat et de la tache (le substrat n'est plus dessiné). Sans elle : celle du substrat, et celle de la
 * tache si le substrat du scénario en a une. Celle des obstacles, dans les deux cas, s'il y a des obstacles.
 */
function verifierLesLegendes(page, scenario, enDensite, contexte) {
  egal(page.el("legende-densite").hidden, !enDensite, `${contexte} : la légende de la densité est cachée`);
  egal(page.el("legende-substrat").hidden, enDensite, `${contexte} : la légende du substrat est cachée`);
  egal(page.el("legende-tache").hidden, enDensite || !scenario.tache, `${contexte} : la légende de la tache est cachée`);
  egal(page.el("legende-obstacle").hidden, scenario.obstacles === 0, `${contexte} : la légende des obstacles est cachée`);
}

// ---------------------------------------------------------------------------------------------------------------
// Les jeux.
// ---------------------------------------------------------------------------------------------------------------

const lignes = [];

async function principal() {
  mkdirSync(dossierTravail, { recursive: true });
  const dossierPage = mkdtempSync(join(dossierTravail, "page-"));
  try {
    writeFileSync(join(dossierPage, "package.json"), '{"type":"module"}\n');
    copyFileSync(join(dossierDocs, "adresse.js"), join(dossierPage, "adresse.js"));
    await jouer(dossierPage);
  } finally {
    rmSync(dossierPage, { recursive: true, force: true });
  }
  process.stdout.write(`${lignes.join("\n")}${lignes.length > 0 ? "\n" : ""}ok ${verifications}\n`);
}

async function jouer(dossierPage) {
  // --- 1. À l'ouverture, la case est décochée (et seule de son espèce), sa légende cachée, et la page ne lit pas la carte
  // de densité : elle ne coûte rien à qui ne la demande pas.
  for (const scenario of SCENARIOS) {
    const page = await ouvrir(`#scenario=${scenario.nom}&graine=3`, dossierPage);
    const cases = [...page.elements.values()].filter((e) => e.getAttribute("type") === "checkbox");
    egal(cases.length, 1, `${scenario.nom} : le nombre de cases à cocher`);
    egal(cases[0].id, "densite", `${scenario.nom} : l'identifiant de la case à cocher`);
    egal(page.el("densite").checked, false, `${scenario.nom} : la case est cochée à l'ouverture`);
    page.images(5);
    verifierLesLegendes(page, scenario, false, `${scenario.nom}, à l'ouverture`);
    egal(page.compteurs.mycelium_densite ?? 0, 0, `${scenario.nom} : décochée, la page lit la densité`);
    egal(page.compteurs.mycelium_densite_max ?? 0, 0, `${scenario.nom} : décochée, la page lit le maximum de la densité`);
  }

  // --- 2. Cochée au temps 0 : la carte est vide (son maximum vaut 0), la couche n'a que la couleur du fond, sans `NaN`.
  for (const scenario of SCENARIOS) {
    const page = await ouvrir(`#scenario=${scenario.nom}&graine=3`, dossierPage);
    page.canvas().journalCourant = [];
    page.cliquer("densite");
    const image = verifierLaDensite(page, `${scenario.nom}, au temps 0`);
    egal(page.u32("mycelium_temps"), 0, `${scenario.nom} : cocher a fait avancer le temps`);
    egal(page.exportations.mycelium_densite_max(), 0, `${scenario.nom} : le maximum de la carte vide`);
    let fonds = 0;
    for (let i = 0; i < image.pixels.length; i += 4) {
      fonds += image.pixels[i] === FOND[0] && image.pixels[i + 1] === FOND[1] && image.pixels[i + 2] === FOND[2] ? 1 : 0;
    }
    egal(fonds, scenario.boite * scenario.boite, `${scenario.nom} : cases au fond, au temps 0`);
    lignes.push(`couche=${scenario.nom},3,0,${hache(image.pixels)}`);
  }

  // --- 3. Cochée en pause au pas de référence, puis décochée : le dessin d'avant revient exactement.
  for (const scenario of SCENARIOS) {
    const graine = 31;
    const page = await ouvrir(`#scenario=${scenario.nom}&graine=${graine}`, dossierPage);
    const m = page.exportations;
    const boite = scenario.boite;

    // Le dessin d'avant, décoché, au pas de référence (la page est alors en pause).
    page.canvas().journalCourant = [];
    page.cliquer("pas-reference");
    egal(page.u32("mycelium_temps"), scenario.duree, `${scenario.nom} : le pas de référence`);
    const avant = dessin(page);
    egal(page.compteurs.mycelium_densite ?? 0, 0, `${scenario.nom} : décochée, la page lit la densité`);

    // Cocher : la légende se montre, la page redessine tout de suite, sans faire avancer le temps.
    page.canvas().journalCourant = [];
    page.compteurs = {};
    page.cliquer("densite");
    egal(page.el("densite").checked, true, `${scenario.nom} : la case ne s'est pas cochée`);
    verifierLesLegendes(page, scenario, true, `${scenario.nom}, cochée au pas de référence`);
    egal(page.u32("mycelium_temps"), scenario.duree, `${scenario.nom} : cocher a fait avancer le temps`);
    const densite = verifierLaDensite(page, `${scenario.nom}, cochée au pas de référence`);
    lignes.push(`couche=${scenario.nom},${graine},${scenario.duree},${hache(densite.pixels)}`);

    // Au plus `largeur × hauteur` cases lues, et un seul maximum, pour une image : la page ne recalcule rien par case.
    const lectures = page.compteurs.mycelium_densite ?? 0;
    verifier(lectures > 0 && lectures <= boite * boite, `${scenario.nom} : ${lectures} lectures de cases pour une image de ${boite * boite} cases`);
    egal(page.compteurs.mycelium_densite_max ?? 0, 1, `${scenario.nom} : les lectures du maximum pour une image`);

    // Une case dense est plus claire qu'une case vide ; la couleur d'une case vide n'est pas celle du substrat.
    const obstacles = obstaclesDuModule(m);
    let dense = 0;
    let plusDense = 0;
    const vides = [];
    for (let i = 0; i < boite * boite; i++) {
      const [x, y] = [i % boite, Math.floor(i / boite)];
      const d = m.mycelium_densite(x, y);
      if (d > plusDense) {
        plusDense = d;
        dense = i;
      }
      if (d === 0 && !obstacles.some(([x0, y0, x1, y1]) => x >= x0 && x <= x1 && y >= y0 && y <= y1)) {
        vides.push(i);
      }
    }
    verifier(plusDense > 0 && vides.length > 0, `${scenario.nom} : il faut une case dense et une case vide`);
    verifier(
      lumiere(densite.pixels, dense) > lumiere(densite.pixels, vides[0]),
      `${scenario.nom} : la case dense n'est pas plus claire que la case vide`,
    );
    egal([...densite.pixels.slice(4 * dense, 4 * dense + 3)].join(), RAMPE_CLAIRE.join(), `${scenario.nom} : la case la plus dense`);
    let memeCouleurQueLeSubstrat = 0;
    for (const i of vides) {
      const [r, g, b] = [0, 1, 2].map((c) => densite.pixels[4 * i + c] === avant.pixels[4 * i + c]);
      memeCouleurQueLeSubstrat += r && g && b ? 1 : 0;
    }
    egal(memeCouleurQueLeSubstrat, 0, `${scenario.nom} : des cases vides ont la couleur du substrat`);

    // Le dessin : la boîte reçoit la couche en cases nettes (sans lissage), puis les obstacles, comme d'habitude ; ni
    // hyphes, ni vignettage ; la bordure et ce qui suit sont ceux d'avant.
    const { dans, apres } = decouper(densite.entrees);
    const images = dans.filter((e) => e.appel === "drawImage");
    egal(images.length, 1, `${scenario.nom} : le nombre d'images posées sur la boîte`);
    egal(images[0].lissage, false, `${scenario.nom} : le lissage de l'image de densité`);
    verifier(
      dans.every((e) => ["drawImage", "fillRect", "strokeRect"].includes(e.appel)),
      `${scenario.nom} : la boîte reçoit autre chose que l'image et les obstacles : ${[...new Set(dans.map((e) => e.appel))]}`,
    );
    egal(dans[0].appel, "drawImage", `${scenario.nom} : l'image de densité n'est pas dessinée avant les obstacles`);
    // Un aplat sur la boîte (le vignettage) serait un `fillRect` de plus que les obstacles : il n'y en a pas.
    egal(dans.filter((e) => e.appel === "fillRect").length, scenario.obstacles, `${scenario.nom} : les aplats dans la boîte`);
    egal(dans.filter((e) => e.appel === "strokeRect").length, scenario.obstacles, `${scenario.nom} : les liserés dans la boîte`);
    // Les obstacles, remplissage et liseré (position, taille, couleur), sont ceux du dessin d'avant.
    const obstaclesDans = (entrees) =>
      entrees.filter((e) => ["fillRect", "strokeRect"].includes(e.appel) && e.args[2] < page.el("boite").width / 2).map(forme);
    egal(
      JSON.stringify(obstaclesDans(dans)),
      JSON.stringify(obstaclesDans(decouper(avant.entrees).dans)),
      `${scenario.nom} : les obstacles dessinés`,
    );
    egal(densite.obstacles.length, scenario.obstacles, `${scenario.nom} : le nombre d'obstacles dessinés`);
    verifier(
      densite.obstacles.every((o) => o.style === COULEUR_OBSTACLE),
      `${scenario.nom} : un obstacle n'a plus la couleur de la page`,
    );
    egal(JSON.stringify(apres.map(forme)), JSON.stringify(decouper(avant.entrees).apres.map(forme)), `${scenario.nom} : la bordure`);
    egal(avant.entrees.find((e) => e.appel === "drawImage")?.lissage, true, `${scenario.nom} : décochée, l'image est lissée`);

    // Décocher : la légende se cache et le dessin d'avant revient exactement (les appels et les pixels de la couche).
    page.canvas().journalCourant = [];
    page.cliquer("densite");
    egal(page.el("densite").checked, false, `${scenario.nom} : la case ne s'est pas décochée`);
    verifierLesLegendes(page, scenario, false, `${scenario.nom}, décochée au pas de référence`);
    const apresDecoche = dessin(page);
    egal(JSON.stringify(apresDecoche.formes), JSON.stringify(avant.formes), `${scenario.nom} : décocher ne redonne pas les appels d'avant`);
    egal(premierEcart(apresDecoche.pixels, avant.pixels), -1, `${scenario.nom} : décocher ne redonne pas le substrat d'avant`);

    // Cocher de nouveau : le même dessin de densité qu'avant (la page ne garde rien de la première fois).
    page.canvas().journalCourant = [];
    page.cliquer("densite");
    const encore = dessin(page);
    egal(JSON.stringify(encore.formes), JSON.stringify(densite.formes), `${scenario.nom} : recocher ne redonne pas les appels`);
    egal(premierEcart(encore.pixels, densite.pixels), -1, `${scenario.nom} : recocher ne redonne pas la même couche`);
  }

  // --- 4. Cochée en marche : la carte se redessine à chaque image, sur la colonie qui avance ; en pause, la page garde
  // la densité ; décochée en pause, elle revient au substrat.
  {
    const page = await ouvrir("#scenario=libre&graine=7", dossierPage);
    page.el("vitesse").value = "4";
    page.el("vitesse").dispatch("input");
    page.cliquer("densite");
    let precedente = hache(verifierLaDensite(page, "en marche, avant la première image").pixels);
    for (let i = 1; i <= 4; i++) {
      page.compteurs = {};
      page.canvas().journalCourant = [];
      page.images(1);
      egal(page.u32("mycelium_temps"), 4 * i, `en marche, image ${i} : le temps`);
      const image = verifierLaDensite(page, `en marche, image ${i}`);
      const courante = hache(image.pixels);
      verifier(courante !== precedente, `en marche, image ${i} : la carte n'a pas changé depuis l'image d'avant`);
      precedente = courante;
      verifier((page.compteurs.mycelium_densite ?? 0) <= 161 * 161, `en marche, image ${i} : trop de lectures de cases`);
      egal(page.compteurs.mycelium_densite_max ?? 0, 1, `en marche, image ${i} : les lectures du maximum`);
    }
    // En pause, la page garde la densité : l'image refaite au moment de la pause est celle de la carte.
    page.canvas().journalCourant = [];
    page.cliquer("pause");
    verifierLaDensite(page, "mise en pause");
    // Décochée en pause : le substrat et les hyphes reviennent, lissés, avec la légende cachée.
    page.canvas().journalCourant = [];
    page.cliquer("densite");
    const substrat = dessin(page);
    egal(substrat.entrees.find((e) => e.appel === "drawImage")?.lissage, true, "décochée en pause : l'image est lissée");
    verifier(decouper(substrat.entrees).dans.some((e) => e.appel === "stroke"), "décochée en pause : aucune hyphe tracée");
    verifier(premierEcart(substrat.pixels, imageDeDensite(page.exportations)) !== -1, "décochée en pause : la couche est encore celle de la densité");
  }

  // --- 5. Cochée, un changement de scénario et « Relancer » repartent du temps 0 : la carte est vide, et la case reste
  // cochée avec sa légende ; « Jusqu'au pas N » dessine la carte du pas de référence.
  {
    const page = await ouvrir("#scenario=libre&graine=7", dossierPage);
    page.cliquer("densite");
    page.images(3);
    verifierLaDensite(page, "libre, avant de changer de scénario");

    page.choisirDansLeMenu("mur");
    egal(page.el("densite").checked, true, "changer de scénario a décoché la case");
    verifierLesLegendes(page, SCENARIOS[4], true, "mur, après le changement de scénario");
    egal(page.u32("mycelium_temps"), 0, "mur : le temps après le changement de scénario");
    egal(page.u32("mycelium_largeur"), 100, "mur : la boîte");
    const vide = verifierLaDensite(page, "mur, au temps 0");
    egal(vide.pixels.length, 4 * 100 * 100, "mur : la taille de la couche");

    page.images(4);
    verifier(page.u32("mycelium_nb_segments") > 0, "mur : la colonie n'a rien laissé");
    verifierLaDensite(page, "mur, après 4 images");

    page.el("graine").value = "99";
    page.cliquer("relancer");
    egal(page.u32("mycelium_temps"), 0, "relancer : le temps");
    egal(hache(verifierLaDensite(page, "relancer").pixels), hache(vide.pixels), "relancer : la carte n'est pas vide");

    page.cliquer("pas-reference");
    egal(page.u32("mycelium_temps"), 120, "mur : le pas de référence");
    const pleine = verifierLaDensite(page, "mur, au pas de référence");
    verifier(hache(pleine.pixels) !== hache(vide.pixels), "mur : la carte du pas de référence est vide");

    // Une adresse changée à la main (`hashchange`) redémarre, case cochée : la carte vide du terrain.
    page.hash = "#scenario=terrain&graine=5";
    for (const fonction of page.ecouteursFenetre.hashchange ?? []) {
      fonction({ type: "hashchange" });
    }
    egal(page.u32("mycelium_temps"), 0, "adresse changée : le temps");
    egal(page.u32("mycelium_nb_obstacles"), 2, "adresse changée : les obstacles du terrain");
    egal(page.el("densite").checked, true, "adresse changée : la case");
    verifierLesLegendes(page, SCENARIOS[1], true, "terrain, après l'adresse changée");
    verifierLaDensite(page, "terrain, après l'adresse changée");
  }

  // --- 6. Cochée avant l'ouverture (un navigateur restitue les cases au rechargement) : la légende se montre dès le
  // départ, et la première image est celle de la densité.
  {
    const page = await ouvrir("#scenario=terrain&graine=3", dossierPage, { cochee: true });
    egal(page.el("densite").checked, true, "à l'ouverture, la case cochée a été décochée");
    verifierLesLegendes(page, SCENARIOS[1], true, "ouverture, case cochée");
    verifierLaDensite(page, "ouverture, case cochée");
    page.images(3);
    verifierLaDensite(page, "ouverture, case cochée, trois images plus tard");
  }

  // --- 7. Les légendes suivent le dessin d'un scénario à l'autre : la tache (qui a une tache), cochée, puis la colonie
  // libre (qui n'en a pas) choisie case cochée, puis décochée : la légende de la tache ne revient pas ; et retour.
  {
    const [libre, tache] = [SCENARIOS[0], SCENARIOS[2]];
    const page = await ouvrir("#scenario=tache&graine=3", dossierPage);
    verifierLesLegendes(page, tache, false, "tache, décochée");
    page.cliquer("densite");
    verifierLesLegendes(page, tache, true, "tache, cochée");
    page.choisirDansLeMenu("libre");
    page.images(2);
    verifierLesLegendes(page, libre, true, "libre, choisie case cochée");
    page.cliquer("densite");
    verifierLesLegendes(page, libre, false, "libre, décochée après le changement");
    page.cliquer("densite");
    page.choisirDansLeMenu("tache");
    page.images(2);
    verifierLesLegendes(page, tache, true, "tache, choisie case cochée");
    page.cliquer("densite");
    verifierLesLegendes(page, tache, false, "tache, décochée après le changement");
  }

  // --- 8. Cochée jusqu'à la fin de la croissance (le mur sans tropisme, graine 1 : plus aucune pointe avant le pas
  // 700) : l'image que la page refait en s'arrêtant est la carte de la colonie finie.
  {
    const page = await ouvrir("#scenario=mur-sans&graine=1", dossierPage);
    page.el("vitesse").value = "16";
    page.el("vitesse").dispatch("input");
    page.cliquer("densite");
    for (let i = 0; i < 60 && !page.el("pause").disabled; i++) {
      page.canvas().journalCourant = [];
      page.images(1);
    }
    egal(page.el("pause").disabled, true, "mur-sans, graine 1 : la colonie n'a pas fini en 60 images");
    egal(page.u32("mycelium_pointes"), 0, "mur-sans, graine 1 : des pointes restent");
    verifierLaDensite(page, "mur-sans, colonie finie");
    verifierLesLegendes(page, SCENARIOS[5], true, "mur-sans, colonie finie");
  }
}

principal().catch((erreur) => {
  process.stderr.write(`page-wasm : ${erreur.message}\n`);
  process.exitCode = 1;
});
