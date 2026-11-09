// Fait tourner la vraie page (`docs/index.html` + `docs/app.js`) dans un faux navigateur, avec le vrai module.
//
// Usage : node page-wasm.mjs <dossier docs> <fichier.wasm> <dossier de travail>
//
// Il n'y a pas de navigateur dans la CI. Ce script en écrit juste assez pour que `app.js` tourne : un faux `document`
// dont les éléments sont ceux que `index.html` déclare (un `getElementById` d'un identifiant absent de la page rend
// `null`, comme dans un navigateur, et `app.js` plante alors), un faux contexte 2D qui note ses appels, une fausse
// adresse (`location.hash`, `history.replaceState`), un faux `fetch` qui lit le module sur le disque et un faux
// `requestAnimationFrame` que le script fait avancer image par image. Le module WebAssembly, lui, est le vrai ;
// `WebAssembly.instantiate` est enveloppé pour tenir le journal de ce que la page lui demande.
//
// Il joue ensuite la page, scénario par scénario, et vérifie ce qu'une personne verrait ou obtiendrait : l'adresse
// lue à l'ouverture, le menu, « Relancer » et « Au hasard », « Jusqu'au pas N », les obstacles dessinés (leur taille en
// cases, bords inclus), le message d'arrêt, l'adresse écrite. Les états sont comparés à ceux d'une autre instance du
// module, pilotée directement. Il ne vérifie ni les couleurs ni la mise en page (le CSS) : un faux navigateur ne les
// calcule pas.
//
// Il écrit `ok <n>` (n : le nombre de vérifications faites) sur la sortie standard s'il n'a rien trouvé à redire ; au
// premier écart, il écrit un message sur la sortie d'erreur et sort avec le code 1.
//
// Aucune dépendance : les modules intégrés de Node seulement.

import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

// Les six scénarios, d'après la spécification : numéro, nom dans l'adresse, boîte, pas de référence, obstacles, et
// s'il a une tache de nutriments (une source plus riche que le substrat de départ : sa légende se montre).
const SCENARIOS = [
  { numero: 0, nom: "libre", boite: 161, duree: 80, obstacles: 0, tache: false },
  { numero: 1, nom: "terrain", boite: 100, duree: 80, obstacles: 2, tache: true },
  { numero: 2, nom: "tache", boite: 100, duree: 100, obstacles: 0, tache: true },
  { numero: 3, nom: "tache-sans", boite: 100, duree: 100, obstacles: 0, tache: true },
  { numero: 4, nom: "mur", boite: 100, duree: 120, obstacles: 1, tache: false },
  { numero: 5, nom: "mur-sans", boite: 100, duree: 120, obstacles: 1, tache: false },
];
const PAR_NOM = Object.fromEntries(SCENARIOS.map((s) => [s.nom, s]));
const MAX_U32 = 4294967295;
const TAILLE_CANVAS = 600;
// Les deux messages d'arrêt, mot pour mot (la spécification les fixe).
const MESSAGE_FIN =
  "La colonie a fini de pousser : plus aucune pointe. « Relancer » ou « Au hasard » en démarre une autre.";
const MESSAGE_LIMITE =
  "Limite de la vitrine atteinte (3000 pas) : la colonie est en pause. « Relancer » en démarre une autre.";

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

/** Un faux contexte 2D : il ne dessine rien et note chaque appel dans `journal`. */
function fauxContexte() {
  const proprietes = {};
  const contexte = new Proxy(
    {},
    {
      get(_cible, nom) {
        if (nom === "journal") {
          return contexte.journalCourant;
        }
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

  click() {
    if (!this.disabled) {
      this.dispatch("click");
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

/** Les huit mots de l'empreinte d'une instance du module, en 64 caractères hexadécimaux. */
function empreinteDe(exportations) {
  let empreinte = "";
  for (let i = 0; i < 8; i++) {
    empreinte += (exportations.mycelium_empreinte_mot(i) >>> 0).toString(16).padStart(8, "0");
  }
  return empreinte;
}

let moduleDeReference = null;

/** L'empreinte, au pas `pas`, de ce scénario et de cette graine, jouée directement sur une autre instance du module. */
async function empreinteDeReference(numero, graine, pas) {
  moduleDeReference ??= (await instancierReel(octetsDuModule, {})).instance.exports;
  moduleDeReference.mycelium_choisir_scenario(numero);
  moduleDeReference.mycelium_demarrer(graine);
  moduleDeReference.mycelium_avancer(pas);
  return empreinteDe(moduleDeReference);
}

/** Ouvre la page avec cette adresse (`hash`) : rend de quoi la piloter et la lire. */
async function ouvrir(hash, dossierPage) {
  numeroDeLancement += 1;
  const elements = elementsDeLaPage(html);
  const page = {
    elements,
    appels: [],
    remplacements: [],
    demandes: [],
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
    replaceState(etat, titre, url) {
      page.remplacements.push([etat, titre, url]);
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
  globalThis.fetch = async (url) => {
    page.demandes.push(String(url));
    return { ok: true, status: 200, arrayBuffer: async () => octetsDuModule };
  };
  WebAssembly.instantiate = async (octets, importations) => {
    const resultat = await instancierReel(octets, importations);
    page.exportations = resultat.instance.exports;
    // Les exportations d'un module sont figées : on rend un objet ordinaire, dont chaque fonction note son appel.
    const enveloppe = {};
    for (const [nom, valeur] of Object.entries(resultat.instance.exports)) {
      enveloppe[nom] =
        typeof valeur === "function"
          ? (...args) => {
              // Pour éprouver la limite de la vitrine (aucune colonie jouée ici ne vit 3000 pas), le script peut faire
              // dire au module qu'il reste une pointe.
              const rendu = nom === "mycelium_pointes" && page.pointesForcees ? 1 : valeur(...args);
              page.appels.push({ nom, args, rendu });
              return rendu;
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
  while (Date.now() < limite && !page.appels.some((a) => a.nom === "mycelium_demarrer")) {
    await new Promise((resolve) => setTimeout(resolve, 2));
  }
  verifier(
    page.appels.some((a) => a.nom === "mycelium_demarrer"),
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
  // Le bouton « Jusqu'au pas N » : son identifiant depuis l'étape 8 (avant : `pas80`).
  page.idPasDeReference = "pas-reference";
  page.derniereAdresse = () => page.remplacements.at(-1)?.[2];
  /** Change l'adresse comme le ferait une personne (un lien collé, une adresse tapée) : `hashchange` sur la fenêtre. */
  page.changerAdresse = (hash) => {
    page.hash = hash;
    for (const fonction of page.ecouteursFenetre.hashchange ?? []) {
      fonction({ type: "hashchange" });
    }
  };
  /** Le texte de tous les éléments de la page, mis bout à bout. */
  page.toutLeTexte = () => [...elements.values()].map((e) => `${e.textContent} ${e.value}`).join(" ");
  page.u32 = (nom, ...args) => page.exportations[nom](...args) >>> 0;
  return page;
}

/** Ce que doit montrer la page : ce scénario, cette graine, au temps `pas`. */
async function verifierColonie(page, scenario, graine, pas, contexte) {
  egal(page.u32("mycelium_duree"), scenario.duree, `${contexte} : la durée du module`);
  egal(page.u32("mycelium_largeur"), scenario.boite, `${contexte} : la largeur du module`);
  egal(page.u32("mycelium_hauteur"), scenario.boite, `${contexte} : la hauteur du module`);
  egal(page.u32("mycelium_nb_obstacles"), scenario.obstacles, `${contexte} : les obstacles du module`);
  egal(page.u32("mycelium_temps"), pas, `${contexte} : le temps du module`);
  egal(empreinteDe(page.exportations), await empreinteDeReference(scenario.numero, graine, pas), `${contexte} : l'état du module`);
  egal(page.el("scenario").value, scenario.nom, `${contexte} : le menu`);
  egal(page.el("graine").value, String(graine), `${contexte} : le champ de graine`);
  egal(page.el(page.idPasDeReference).textContent, `Jusqu'au pas ${scenario.duree}`, `${contexte} : le bouton du pas de référence`);
  egal(page.derniereAdresse(), `#scenario=${scenario.nom}&graine=${graine}`, `${contexte} : l'adresse écrite`);
  egal(page.hash, `#scenario=${scenario.nom}&graine=${graine}`, `${contexte} : l'adresse de la page`);
  verifier(page.el("description-scenario").textContent.trim() !== "", `${contexte} : la description du scénario est vide`);
  egal(page.el("legende-obstacle").hidden, scenario.obstacles === 0, `${contexte} : la légende des obstacles est cachée`);
  egal(page.el("legende-tache").hidden, !scenario.tache, `${contexte} : la légende de la tache est cachée`);
}

// ---------------------------------------------------------------------------------------------------------------
// Les vérifications.
// ---------------------------------------------------------------------------------------------------------------

/** Les rectangles pleins que la page a dessinés sur le canvas, hors les aplats qui couvrent la boîte (le fond, le vignettage). */
function rectanglesDObstacles(page) {
  const vus = new Map();
  for (const entree of page.el("boite").getContext("2d").journalCourant) {
    if (entree.appel !== "fillRect") {
      continue;
    }
    const [x, y, largeur, hauteur] = entree.args;
    if (largeur < page.el("boite").width / 2) {
      vus.set(`${x},${y},${largeur},${hauteur}`, { x, y, largeur, hauteur });
    }
  }
  return [...vus.values()].sort((a, b) => a.y - b.y || a.x - b.x);
}

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
  process.stdout.write(`ok ${verifications}\n`);
}

async function jouer(dossierPage) {
  // --- 1. À l'ouverture, sans adresse : le scénario libre, la graine du champ (1).
  {
    const page = await ouvrir("", dossierPage);
    egal(page.demandes[0], "mycelium.wasm", "le module demandé");
    await verifierColonie(page, PAR_NOM.libre, 1, 0, "ouverture sans adresse");
  }

  // --- 2. À l'ouverture avec une adresse valide : le menu, le champ et la colonie sont ceux de l'adresse.
  const descriptions = new Set();
  for (const scenario of SCENARIOS) {
    for (const graine of [31, 0, MAX_U32]) {
      const page = await ouvrir(`#scenario=${scenario.nom}&graine=${graine}`, dossierPage);
      await verifierColonie(page, scenario, graine, 0, `ouverture sur ${scenario.nom}, graine ${graine}`);
      descriptions.add(page.el("description-scenario").textContent.trim());
    }
  }
  egal(descriptions.size, 6, "le nombre de descriptions différentes des six scénarios");

  // --- 3. Une adresse inconnue ou invalide : « libre » pour le nom, la graine du champ pour la graine.
  const adressesFausses = [
    ["#scenario=inconnu&graine=5", "libre", 5],
    ["#scenario=Mur&graine=5", "libre", 5],
    ["#scenario=constructor&graine=5", "libre", 5],
    ["#scenario=__proto__&graine=5", "libre", 5],
    ["#scenario=mur&graine=abc", "mur", 1],
    ["#scenario=mur&graine=-1", "mur", 1],
    ["#scenario=mur&graine=1.5", "mur", 1],
    ["#scenario=mur&graine=4294967296", "mur", 1],
    ["#scenario=mur&graine=", "mur", 1],
    ["#scenario=mur", "mur", 1],
    ["#graine=9", "libre", 9],
    ["#graine=0", "libre", 0],
    ["#", "libre", 1],
    ["", "libre", 1],
  ];
  for (const [hash, nom, graine] of adressesFausses) {
    const page = await ouvrir(hash, dossierPage);
    await verifierColonie(page, PAR_NOM[nom], graine, 0, `adresse « ${hash} »`);
  }

  // --- 4. Changer le menu choisit le scénario puis redémarre avec la graine du champ, du temps 0.
  {
    const page = await ouvrir("#scenario=libre&graine=7", dossierPage);
    const descriptionsDuMenu = new Set();
    for (const nom of ["mur", "libre", "terrain", "mur-sans", "tache-sans", "tache", "mur"]) {
      page.images(3);
      verifier(page.u32("mycelium_temps") > 0, `avant de passer à ${nom} : la colonie n'a pas avancé`);
      page.choisirDansLeMenu(nom);
      await verifierColonie(page, PAR_NOM[nom], 7, 0, `menu changé pour ${nom}`);
      descriptionsDuMenu.add(page.el("description-scenario").textContent.trim());
    }
    egal(descriptionsDuMenu.size, 6, "le nombre de descriptions vues en changeant le menu");
  }

  // --- 5. « Relancer » et « Au hasard » démarrent le scénario choisi.
  {
    const page = await ouvrir("#scenario=mur&graine=31", dossierPage);
    page.images(5);
    page.el("graine").value = "99";
    page.cliquer("relancer");
    await verifierColonie(page, PAR_NOM.mur, 99, 0, "relancer sur mur");

    page.choisirDansLeMenu("tache");
    page.images(4);
    page.el("graine").value = "4294967295";
    page.cliquer("relancer");
    await verifierColonie(page, PAR_NOM.tache, MAX_U32, 0, "relancer sur tache, grande graine");

    page.choisirDansLeMenu("terrain");
    page.images(4);
    page.cliquer("hasard");
    const graine = Number(page.el("graine").value);
    verifier(Number.isInteger(graine) && graine >= 0 && graine <= MAX_U32, `« Au hasard » : graine ${page.el("graine").value}`);
    await verifierColonie(page, PAR_NOM.terrain, graine, 0, "au hasard sur terrain");
    // Au hasard pour de vrai : trois tirages de 32 bits ne donnent pas tous la même graine (une chance sur 2^64).
    const graines = new Set([String(graine)]);
    for (let i = 0; i < 2; i++) {
      page.cliquer("hasard");
      graines.add(page.el("graine").value);
    }
    verifier(graines.size > 1, "« Au hasard » redonne toujours la même graine");
  }

  // --- 5 (suite). Quand la graine du champ est invalide, changer le menu ne démarre rien : la colonie en cours
  // continue (même temps, même état), le menu revient au scénario qu'elle fait tourner, l'adresse ne change pas et le
  // message dit ce qu'est une graine valide. « Relancer » fait de même, sans toucher au menu.
  for (const invalide of ["", "-5", "4294967296", "1.5", "<b>3</b>"]) {
    const page = await ouvrir("#scenario=libre&graine=7", dossierPage);
    page.images(3);
    const temps = page.u32("mycelium_temps");
    const empreinte = empreinteDe(page.exportations);
    page.el("graine").value = invalide;
    page.choisirDansLeMenu("mur");
    egal(page.el("scenario").value, "libre", `graine « ${invalide} » : le menu ne revient pas au scénario en cours`);
    egal(page.u32("mycelium_temps"), temps, `graine « ${invalide} » : la colonie a redémarré`);
    egal(empreinteDe(page.exportations), empreinte, `graine « ${invalide} » : l'état a changé`);
    egal(page.u32("mycelium_duree"), PAR_NOM.libre.duree, `graine « ${invalide} » : le module a changé de scénario`);
    egal(page.hash, "#scenario=libre&graine=7", `graine « ${invalide} » : l'adresse a changé`);
    egal(page.el("message").textContent, "La graine doit être un entier de 0 à 4294967295.", `graine « ${invalide} » : le message`);
    page.cliquer("relancer");
    egal(page.u32("mycelium_temps"), temps, `graine « ${invalide} » : « Relancer » a redémarré`);
    egal(page.el("message").textContent, "La graine doit être un entier de 0 à 4294967295.", `graine « ${invalide} » : le message de « Relancer »`);
    // Une graine redevenue valide : le menu redémarre de nouveau, et le message s'efface.
    page.el("graine").value = "12";
    page.choisirDansLeMenu("mur");
    await verifierColonie(page, PAR_NOM.mur, 12, 0, `graine « ${invalide} » puis 12`);
    egal(page.el("message").textContent, "", `graine « ${invalide} » puis 12 : le message reste`);
  }

  // --- 5 (suite). Une adresse changée dans l'onglet (`hashchange`) redémarre la colonie qu'elle décrit, avec les mêmes
  // règles qu'à l'ouverture (nom inconnu : « libre » ; graine absente ou invalide : celle du champ).
  {
    const page = await ouvrir("#scenario=libre&graine=7", dossierPage);
    page.images(4);
    page.changerAdresse("#scenario=mur-sans&graine=42");
    await verifierColonie(page, PAR_NOM["mur-sans"], 42, 0, "adresse changée pour mur-sans");
    page.images(4);
    page.changerAdresse("#scenario=terrain");
    await verifierColonie(page, PAR_NOM.terrain, 42, 0, "adresse changée pour terrain, sans graine");
    page.images(4);
    page.changerAdresse("#scenario=inconnu&graine=abc");
    await verifierColonie(page, PAR_NOM.libre, 42, 0, "adresse changée pour un nom inconnu et une graine invalide");
    page.images(4);
    page.changerAdresse("#graine=0&scenario=tache");
    await verifierColonie(page, PAR_NOM.tache, 0, 0, "adresse changée pour tache, graine 0");
  }

  // --- 5 (suite). Une adresse piégée (du HTML, du script) : la page ouvre « libre », et rien de ce texte ne se
  // retrouve dans la page (le faux navigateur refuse en plus toute écriture dans innerHTML).
  for (const [hash, graine] of [
    ["#scenario=<img src=x onerror=alert(1)>&graine=5", 5],
    ["#scenario=mur\"><script>alert(1)</script>&graine=<b>1</b>", 1],
  ]) {
    const page = await ouvrir(hash, dossierPage);
    await verifierColonie(page, PAR_NOM.libre, graine, 0, `adresse piégée « ${hash} »`);
    const texte = page.toutLeTexte();
    for (const morceau of ["<img", "onerror", "<script", "alert", "<b>"]) {
      verifier(!texte.includes(morceau), `adresse piégée « ${hash} » : « ${morceau} » se retrouve dans la page`);
    }
  }

  // --- 5 (suite). Au temps 0, la spore n'est pas dessinée (le module n'en donne pas la position) : aucun cercle
  // (`arc`) sur le canvas ; seuls les coins de la boîte sont arrondis (`arcTo`).
  for (const scenario of SCENARIOS) {
    const page = await ouvrir(`#scenario=${scenario.nom}&graine=3`, dossierPage);
    const cercles = page.el("boite").getContext("2d").journalCourant.filter((e) => e.appel === "arc");
    egal(cercles.length, 0, `${scenario.nom} : des cercles sont dessinés au temps 0`);
  }

  // --- 6. « Jusqu'au pas N » : N est le pas de référence du scénario en cours, atteint puis tenu.
  for (const scenario of SCENARIOS) {
    const page = await ouvrir(`#scenario=${scenario.nom}&graine=1`, dossierPage);
    page.cliquer(page.idPasDeReference);
    await verifierColonie(page, scenario, 1, scenario.duree, `${scenario.nom} : jusqu'au pas de référence`);
    const note = page.el("note-empreinte").textContent;
    verifier(note.includes("natif"), `${scenario.nom} : la note de l'empreinte ne parle pas du natif : « ${note} »`);
    egal(note.includes("couplage-graine-1"), scenario.nom === "libre", `${scenario.nom} : la note cite la référence du couplage`);
    // Au pas de référence, tout « pas N » que dit la note est le pas de référence de ce scénario (et non un 80 resté
    // d'avant les six scénarios). Si la note ne dit aucun « pas N », il n'y a rien à comparer.
    for (const trouve of note.matchAll(/\bpas (\d+)/g)) {
      egal(Number(trouve[1]), scenario.duree, `${scenario.nom} : la note de l'empreinte parle d'un autre pas que le pas de référence : « ${note} »`);
    }

    // Déjà au pas de référence : rien ne bouge.
    page.cliquer(page.idPasDeReference);
    await verifierColonie(page, scenario, 1, scenario.duree, `${scenario.nom} : un second clic sur le pas de référence`);

    // Au-delà : on remet la colonie en marche (le bouton Pause, qui reprend), puis la page, cliquée sur le pas de
    // référence, repart de zéro avec la même graine et revient au pas de référence.
    page.cliquer("pause");
    page.el("vitesse").value = "16";
    page.el("vitesse").dispatch("input");
    page.images(Math.ceil(scenario.duree / 16) + 3);
    verifier(page.u32("mycelium_temps") > scenario.duree, `${scenario.nom} : la colonie devait dépasser le pas de référence`);
    page.cliquer(page.idPasDeReference);
    await verifierColonie(page, scenario, 1, scenario.duree, `${scenario.nom} : le pas de référence depuis un temps plus avancé`);
  }

  // --- 7. Les obstacles se dessinent : un rectangle par obstacle, de x0 à x1 inclus et de y0 à y1 inclus.
  for (const scenario of SCENARIOS) {
    const page = await ouvrir(`#scenario=${scenario.nom}&graine=1`, dossierPage);
    page.images(3);
    page.el("boite").getContext("2d").journalCourant = [];
    page.choisirDansLeMenu(scenario.nom);
    page.images(2);
    const rectangles = rectanglesDObstacles(page);
    egal(rectangles.length, scenario.obstacles, `${scenario.nom} : le nombre d'obstacles dessinés`);
    if (scenario.obstacles === 0) {
      continue;
    }
    // Ce que dit le module : les cases de chaque obstacle, bords inclus.
    const cases = [];
    for (let i = 0; i < scenario.obstacles; i++) {
      const [x0, y0, x1, y1] = [0, 1, 2, 3].map((k) => page.exportations.mycelium_obstacle(i, k));
      cases.push({ x0, y0, x1, y1, largeur: x1 - x0 + 1, hauteur: y1 - y0 + 1 });
    }
    // La taille d'une case à l'écran : quand deux obstacles sont à deux hauteurs différentes, leur écart la donne ;
    // avec un seul, on vérifie que sa forme est celle de ses cases (largeur sur hauteur) et que la boîte entière tient
    // dans le canvas sans le remplir à moitié.
    const dessines = [...rectangles].sort((a, b) => a.y - b.y);
    const attendus = [...cases].sort((a, b) => a.y0 - b.y0);
    let case_;
    if (attendus.length >= 2 && attendus[1].y0 !== attendus[0].y0) {
      case_ = (dessines[1].y - dessines[0].y) / (attendus[1].y0 - attendus[0].y0);
    } else {
      case_ = dessines[0].hauteur / attendus[0].hauteur;
      const forme = dessines[0].largeur / dessines[0].hauteur;
      const attendue = attendus[0].largeur / attendus[0].hauteur;
      verifier(Math.abs(forme - attendue) < 0.01 * attendue, `${scenario.nom} : forme de l'obstacle ${forme}, attendue ${attendue} (${attendus[0].largeur} cases sur ${attendus[0].hauteur})`);
    }
    verifier(case_ * scenario.boite <= TAILLE_CANVAS && case_ * scenario.boite >= TAILLE_CANVAS / 2, `${scenario.nom} : la case mesure ${case_} pixels, la boîte ne tient pas dans le canvas`);
    const tolerance = 0.15 * case_;
    for (let i = 0; i < attendus.length; i++) {
      verifier(
        Math.abs(dessines[i].largeur - attendus[i].largeur * case_) <= tolerance,
        `${scenario.nom} : l'obstacle ${i} est large de ${dessines[i].largeur} pixels, ${attendus[i].largeur} cases de ${case_} en font ${attendus[i].largeur * case_}`,
      );
      verifier(
        Math.abs(dessines[i].hauteur - attendus[i].hauteur * case_) <= tolerance,
        `${scenario.nom} : l'obstacle ${i} est haut de ${dessines[i].hauteur} pixels, ${attendus[i].hauteur} cases de ${case_} en font ${attendus[i].hauteur * case_}`,
      );
    }
    // La position, quand la page montre le substrat par une image carrée posée sur le canvas (`drawImage` à cinq
    // arguments : image, x, y, largeur, hauteur) : cette image est la boîte, et chaque obstacle est à sa case.
    const images = page.el("boite").getContext("2d").journalCourant.filter((e) => e.appel === "drawImage" && e.args.length === 5);
    if (images.length > 0) {
      const [, boiteX, boiteY, boiteLargeur, boiteHauteur] = images.at(-1).args;
      verifier(Math.abs(boiteLargeur - boiteHauteur) < 1, `${scenario.nom} : la boîte dessinée n'est pas carrée`);
      const unite = boiteLargeur / scenario.boite;
      for (let i = 0; i < attendus.length; i++) {
        verifier(
          Math.abs(dessines[i].x - (boiteX + attendus[i].x0 * unite)) <= 0.15 * unite,
          `${scenario.nom} : l'obstacle ${i} est à ${dessines[i].x} pixels de gauche, sa colonne ${attendus[i].x0} est à ${boiteX + attendus[i].x0 * unite}`,
        );
        verifier(
          Math.abs(dessines[i].y - (boiteY + attendus[i].y0 * unite)) <= 0.15 * unite,
          `${scenario.nom} : l'obstacle ${i} est à ${dessines[i].y} pixels du haut, sa ligne ${attendus[i].y0} est à ${boiteY + attendus[i].y0 * unite}`,
        );
        verifier(
          Math.abs(dessines[i].largeur - attendus[i].largeur * unite) <= 0.15 * unite &&
            Math.abs(dessines[i].hauteur - attendus[i].hauteur * unite) <= 0.15 * unite,
          `${scenario.nom} : l'obstacle ${i} ne fait pas ${attendus[i].largeur} sur ${attendus[i].hauteur} cases de ${unite} pixels`,
        );
      }
    }
    // Deux obstacles de même colonne de départ : même bord gauche à l'écran.
    if (attendus.length >= 2 && attendus[0].x0 === attendus[1].x0) {
      verifier(Math.abs(dessines[0].x - dessines[1].x) <= tolerance, `${scenario.nom} : les deux obstacles ne sont pas alignés`);
    }

    // L'ordre, sur une image : le substrat (l'image carrée), puis les obstacles, puis les hyphes (un tracé `lineTo`
    // suivi d'un `stroke`), le tout avant le `restore` qui ferme le dessin de la boîte.
    page.el("boite").getContext("2d").journalCourant = [];
    page.images(1);
    verifier(page.u32("mycelium_nb_segments") > 0, `${scenario.nom} : aucune hyphe à dessiner pour vérifier l'ordre`);
    const journal = page.el("boite").getContext("2d").journalCourant;
    const rang = (predicat) => journal.findIndex(predicat);
    const dernierRang = (predicat) => journal.findLastIndex(predicat);
    const estObstacle = (e) => e.appel === "fillRect" && e.args[2] < page.el("boite").width / 2;
    const substrat = dernierRang((e) => e.appel === "drawImage");
    const premierObstacle = rang(estObstacle);
    const dernierObstacle = Math.max(dernierRang(estObstacle), dernierRang((e) => e.appel === "strokeRect"));
    const fin = rang((e) => e.appel === "restore");
    const traceHyphes = journal.findIndex((e, i) => i > dernierObstacle && e.appel === "lineTo");
    const traitHyphes = journal.findIndex((e, i) => i > traceHyphes && e.appel === "stroke");
    verifier(substrat !== -1 && premierObstacle > substrat, `${scenario.nom} : les obstacles ne sont pas dessinés après le substrat`);
    verifier(traceHyphes !== -1 && traitHyphes !== -1 && traitHyphes < fin, `${scenario.nom} : les hyphes ne sont pas tracées après les obstacles`);
    // Entre le substrat et les obstacles, aucun tracé (le chemin des coins arrondis, lui, précède le substrat).
    verifier(journal.slice(substrat, premierObstacle).every((e) => e.appel !== "lineTo"),
      `${scenario.nom} : des hyphes sont tracées avant les obstacles (elles seraient cachées)`);
  }

  // --- 8. Le message d'arrêt, dans l'élément d'état.
  {
    const page = await ouvrir("#scenario=libre&graine=7", dossierPage);
    page.el("vitesse").value = "16";
    page.el("vitesse").dispatch("input");
    for (let i = 0; i < 300 && !page.el("etat").textContent.includes("fini de pousser"); i++) {
      page.images(1);
    }
    egal(page.el("etat").textContent, MESSAGE_FIN, "libre, graine 7 : le message d'arrêt");
    egal(page.u32("mycelium_pointes"), 0, "libre, graine 7 : les pointes à la fin");
    egal(page.el("pause").disabled, true, "libre, graine 7 : le bouton Pause une fois la colonie finie");
    // Relancer la remet en marche.
    page.cliquer("relancer");
    egal(page.el("pause").disabled, false, "après Relancer : le bouton Pause");
    verifier(!page.el("etat").textContent.includes("fini de pousser"), "après Relancer : le message d'arrêt reste affiché");
  }
  // Le mur sans tropisme, graine 1, finit de pousser vers le pas 640 : le message d'arrêt et la colonie figée.
  {
    const page = await ouvrir("#scenario=mur-sans&graine=1", dossierPage);
    page.el("vitesse").value = "16";
    page.el("vitesse").dispatch("input");
    for (let i = 0; i < 400 && !page.el("pause").disabled; i++) {
      page.images(1);
    }
    egal(page.el("pause").disabled, true, "mur-sans : la colonie ne s'est jamais arrêtée");
    egal(page.u32("mycelium_pointes"), 0, "mur-sans : les pointes à la fin");
    egal(page.el("etat").textContent, MESSAGE_FIN, "mur-sans : le message d'arrêt");
    const temps = page.u32("mycelium_temps");
    page.images(5);
    egal(page.u32("mycelium_temps"), temps, "mur-sans : la colonie figée avance encore");
  }
  // La limite de la vitrine : une colonie qui aurait encore des pointes au pas 3000 s'y fige, exactement, avec son
  // message. Le script fait dire au module qu'il reste une pointe (aucune colonie jouée ici ne vit si longtemps).
  {
    const page = await ouvrir("#scenario=tache&graine=2", dossierPage);
    page.pointesForcees = true;
    page.el("vitesse").value = "16";
    page.el("vitesse").dispatch("input");
    for (let i = 0; i < 400 && !page.el("pause").disabled; i++) {
      page.images(1);
    }
    egal(page.el("pause").disabled, true, "limite : la colonie ne s'est jamais arrêtée");
    egal(page.u32("mycelium_temps"), 3000, "limite : le temps à la limite de la vitrine");
    egal(page.el("etat").textContent, MESSAGE_LIMITE, "limite : le message d'arrêt");
    page.images(5);
    egal(page.u32("mycelium_temps"), 3000, "limite : la colonie dépasse la limite");
    page.pointesForcees = false;
  }
}

principal().catch((erreur) => {
  process.stderr.write(`page-wasm : ${erreur.message}\n`);
  process.exitCode = 1;
});
