/*
  La vitrine de Mycelium Lab.

  La page charge le module WebAssembly `mycelium.wasm`, choisit un des six scénarios, démarre la
  simulation avec une graine, puis l'avance d'un certain nombre de pas à chaque image et la dessine
  sur le canvas. Le module n'importe rien : toutes les fonctions qu'il exporte ne prennent et ne
  rendent que des nombres.

  Le scénario et la graine se lisent dans l'adresse de la page (`#scenario=<nom>&graine=<n>`, voir
  adresse.js) et s'y réécrivent à chaque démarrage : une adresse recopiée rejoue la même colonie.

  Aucune ressource externe : pas de police, pas d'image, pas de bibliothèque. Le rendu est fait
  avec le contexte 2D du canvas ; la simulation est le module lui-même.

  L'empreinte de l'état est chère à lire (huit appels, chacun hache tout l'état), donc on ne la
  rafraîchit qu'à intervalles espacés, et toujours en pause, au redémarrage ou au saut au pas de référence.
*/

import { GRAINE_MAX, ecrireAdresse, lireAdresse, lireGraine, numeroDuScenario } from "./adresse.js";

// Ce que la page dit de chaque scénario, par son nom dans l'adresse.
const DESCRIPTIONS = {
  libre: "Une colonie libre dans une boîte de 161 × 161 cases : elle pousse, se ramifie et s'anastomose.",
  terrain:
    "Une tache de nutriments à droite et deux murs au milieu, avec un passage : la colonie cherche le passage.",
  tache:
    "Une tache de nutriments près de la spore : les hyphes se penchent vers elle (chimiotropisme 10).",
  "tache-sans": "Le même scénario sans chimiotropisme : à comparer avec le précédent.",
  mur:
    "Un mur devant la spore, thigmotropisme 35 : les hyphes qui le touchent de biais le longent, " +
    "celles qui le frappent de face s'arrêtent et se ramifient.",
  "mur-sans": "Le même mur sans thigmotropisme : une pointe qui touche le mur y reste collée.",
};

// La graine du champ à la première ouverture, quand l'adresse n'en donne pas de valide.
const GRAINE_PAR_DEFAUT = 1;

// La vitrine n'a pas à tourner indéfiniment : au-delà de 3000 pas, on fige la colonie même si
// des pointes subsistent, pour qu'une page laissée ouverte ne consomme plus rien.
const PAS_MAX = 3000;

// Le substrat n'est consommé qu'à quelques millièmes par pas : on étale cet écart pour que la
// déplétion se voie, sans jamais modifier les valeurs lues dans le module.
const CONTRASTE_SUBSTRAT = 60;

// Une case plus riche que le substrat de départ (une tache de nutriments) se teinte : à partir de
// cet excédent sur 1, elle est teintée à fond.
const EXCEDENT_PLEIN = 4;

// Les obstacles : un remplissage ardoise et un liseré plus clair, bien distincts du fond vert sombre
// et des hyphes crème. Ces deux couleurs sont aussi celles de l'échantillon de la légende.
const COULEUR_OBSTACLE = "#5b6b73";
const COULEUR_LISERE_OBSTACLE = "#9fb0b8";

// Périodes de rafraîchissement, en images affichées (et non en pas de simulation).
const IMAGES_PAR_SUBSTRAT = 4;
const IMAGES_PAR_EMPREINTE = 90;

// Les éléments de la page, tous désignés par leur identifiant.
const canvas = document.getElementById("boite");
const contexte = canvas.getContext("2d");

const selecteur = document.getElementById("scenario");
const descriptionScenario = document.getElementById("description-scenario");
const legendeObstacle = document.getElementById("legende-obstacle");
const legendeTache = document.getElementById("legende-tache");

const champGraine = document.getElementById("graine");
const boutonHasard = document.getElementById("hasard");
const boutonRelancer = document.getElementById("relancer");
const boutonPause = document.getElementById("pause");
const boutonPasReference = document.getElementById("pas-reference");
const curseurVitesse = document.getElementById("vitesse");
const sortieVitesse = document.getElementById("vitesse-valeur");

const message = document.getElementById("message");
const etat = document.getElementById("etat");

const champTemps = document.getElementById("temps");
const champBiomasse = document.getElementById("biomasse");
const champPointes = document.getElementById("pointes");
const champAnastomoses = document.getElementById("anastomoses");
const champEmpreinte = document.getElementById("empreinte");
const noteEmpreinte = document.getElementById("note-empreinte");

// L'état de la page. `module` reste nul tant que le fichier WebAssembly n'est pas chargé.
let module = null;
let enPause = false;
// Vrai quand la colonie a fini de pousser (ou a atteint PAS_MAX) : le bouton Pause reste alors
// désactivé jusqu'au prochain démarrage.
let colonieFinie = false;
// La graine et le scénario de la colonie qui pousse (le menu et le champ peuvent déjà dire autre chose).
let graineCourante = GRAINE_PAR_DEFAUT;
let scenarioCourant = "libre";
let compteurImages = 0;
let substratAJour = false;

// Une couche hors écran à la résolution de la grille : le substrat s'y dessine case par case,
// puis elle est agrandie sur le canvas, ce qui donne des dégradés doux.
let coucheSubstrat = document.createElement("canvas");
let contexteSubstrat = coucheSubstrat.getContext("2d");
let pixelsSubstrat = null;

const nombreFr = new Intl.NumberFormat("fr-FR", { maximumFractionDigits: 2 });

// ---------------------------------------------------------------------------------------------
// Le chargement du module
// ---------------------------------------------------------------------------------------------

async function charger() {
  try {
    const reponse = await fetch("mycelium.wasm");
    if (!reponse.ok) {
      throw new Error(`fichier introuvable (statut ${reponse.status})`);
    }
    const octets = await reponse.arrayBuffer();
    // Le module n'importe rien : la liste d'importations est vide.
    const { instance } = await WebAssembly.instantiate(octets, {});
    module = instance.exports;
    ajusterTaille();
    // Le menu et le champ disent déjà ce que l'adresse demandait (voir appliquerAdresse) : on démarre dessus.
    demarrer(graineLue() ?? GRAINE_PAR_DEFAUT, selecteur.value);
  } catch (erreur) {
    etat.textContent = "Le module n'a pas pu être chargé.";
    message.textContent =
      `Erreur : ${erreur.message}. Servez la page depuis le dossier docs avec un petit serveur local.`;
    dessiner();
  }
}

// ---------------------------------------------------------------------------------------------
// Les commandes
// ---------------------------------------------------------------------------------------------

function pret() {
  if (module !== null) {
    return true;
  }
  message.textContent = "Le module est encore en chargement.";
  return false;
}

// Lit le champ de graine ; rend null si le texte n'est pas un entier de 0 à 4294967295.
function graineLue() {
  return lireGraine(champGraine.value.trim());
}

// Lit l'adresse de la page et la met dans le menu, le champ de graine et la description (sans rien démarrer).
// Un nom absent ou inconnu donne « libre » ; une graine absente ou invalide laisse celle du champ.
function appliquerAdresse() {
  const adresse = lireAdresse(window.location.hash, graineLue() ?? GRAINE_PAR_DEFAUT);
  selecteur.value = adresse.scenario;
  champGraine.value = String(adresse.graine);
  descriptionScenario.textContent = DESCRIPTIONS[adresse.scenario];
}

function graineAleatoire() {
  if (window.crypto && typeof window.crypto.getRandomValues === "function") {
    const tampon = new Uint32Array(1);
    window.crypto.getRandomValues(tampon);
    return tampon[0];
  }
  return Math.floor(Math.random() * (GRAINE_MAX + 1)) >>> 0;
}

function vitesse() {
  const valeur = Number.parseInt(curseurVitesse.value, 10);
  return Number.isInteger(valeur) && valeur >= 1 ? valeur : 1;
}

function majBoutonPause() {
  boutonPause.textContent = enPause ? "Reprendre" : "Pause";
  boutonPause.setAttribute("aria-pressed", enPause ? "true" : "false");
}

// Démarre une nouvelle simulation de ce scénario (son nom dans l'adresse) avec cette graine, en repartant
// du temps 0.
function demarrer(graine, scenario) {
  // Le scénario se choisit d'abord : le module construit celui du dernier choix au prochain démarrage.
  if (module.mycelium_choisir_scenario(numeroDuScenario(scenario)) !== 1) {
    message.textContent = `Scénario inconnu : « ${scenario} ».`;
    return;
  }
  graineCourante = graine;
  scenarioCourant = scenario;
  selecteur.value = scenario;
  champGraine.value = String(graine);
  descriptionScenario.textContent = DESCRIPTIONS[scenario];
  module.mycelium_demarrer(graine);
  // Le bouton annonce le pas de référence du scénario qui vient de démarrer.
  boutonPasReference.textContent = `Jusqu'au pas ${module.mycelium_duree() >>> 0}`;
  legendeObstacle.hidden = (module.mycelium_nb_obstacles() >>> 0) === 0;
  ecrireAdresseCourante();
  enPause = false;
  colonieFinie = false;
  compteurImages = 0;
  substratAJour = false;
  majBoutonPause();
  // Une nouvelle colonie peut de nouveau être mise en pause.
  boutonPause.disabled = false;
  boutonPause.setAttribute("aria-disabled", "false");
  dessiner();
  afficherMesures();
  rafraichirEmpreinte();
  etat.textContent = "La colonie pousse.";
  message.textContent = "";
}

// Écrit dans l'adresse le scénario et la graine de la colonie qui pousse, sans ajouter de ligne à l'historique.
function ecrireAdresseCourante() {
  try {
    history.replaceState(null, "", ecrireAdresse(scenarioCourant, graineCourante));
  } catch (erreur) {
    // Une page sans droit sur l'historique (cadre isolé) ne perd que son adresse, pas la simulation.
  }
}

boutonRelancer.addEventListener("click", () => {
  if (!pret()) {
    return;
  }
  const graine = graineLue();
  if (graine === null) {
    message.textContent = `La graine doit être un entier de 0 à ${GRAINE_MAX}.`;
    return;
  }
  demarrer(graine, selecteur.value);
});

boutonHasard.addEventListener("click", () => {
  if (!pret()) {
    return;
  }
  demarrer(graineAleatoire(), selecteur.value);
});

// Changer de scénario redémarre aussitôt, avec la graine du champ comme « Relancer ». Si cette graine est
// invalide, rien ne démarre : le menu revient au scénario qui pousse, pour qu'il ne mente pas.
selecteur.addEventListener("change", () => {
  if (!pret()) {
    return;
  }
  const graine = graineLue();
  if (graine === null) {
    selecteur.value = scenarioCourant;
    message.textContent = `La graine doit être un entier de 0 à ${GRAINE_MAX}.`;
    return;
  }
  demarrer(graine, selecteur.value);
});

// Une adresse changée à la main (ou un lien collé dans l'onglet) fait repartir la colonie qu'elle décrit.
window.addEventListener("hashchange", () => {
  appliquerAdresse();
  if (module !== null) {
    demarrer(graineLue() ?? GRAINE_PAR_DEFAUT, selecteur.value);
  }
});

champGraine.addEventListener("keydown", (evenement) => {
  if (evenement.key === "Enter") {
    evenement.preventDefault();
    boutonRelancer.click();
  }
});

boutonPause.addEventListener("click", () => {
  if (!pret()) {
    return;
  }
  if (colonieFinie) {
    // La colonie est figée : seule une nouvelle graine la relance.
    return;
  }
  enPause = !enPause;
  majBoutonPause();
  if (enPause) {
    // L'image, les mesures et l'empreinte sont remises à jour au moment où l'on fige la colonie.
    substratAJour = false;
    dessiner();
    afficherMesures();
    rafraichirEmpreinte();
    etat.textContent = "En pause.";
  } else {
    etat.textContent = "La colonie pousse.";
  }
});

boutonPasReference.addEventListener("click", () => {
  if (!pret()) {
    return;
  }
  // Le pas de référence est la durée du scénario de la colonie qui pousse.
  const reference = module.mycelium_duree() >>> 0;
  const temps = module.mycelium_temps() >>> 0;
  let passe;
  if (temps < reference) {
    module.mycelium_avancer(reference - temps);
    passe = `Pas ${reference} atteint : la colonie est en pause.`;
  } else if (temps === reference) {
    // Déjà sur la référence : il n'y a rien à avancer.
    passe = `La colonie est déjà au pas ${reference} : elle reste en pause.`;
  } else {
    // La simulation a dépassé la référence : on repart de zéro avec la même graine puis on
    // avance exactement de `reference` pas ; le déterminisme redonne l'état de référence.
    demarrer(graineCourante, scenarioCourant);
    module.mycelium_avancer(reference);
    passe = `La colonie dépassait le pas ${reference} : repartie de zéro, elle est revenue au pas ${reference} et reste en pause.`;
  }
  // On s'arrête au pas de la référence : l'empreinte affichée est celle que donne le natif.
  enPause = true;
  majBoutonPause();
  substratAJour = false;
  dessiner();
  afficherMesures();
  rafraichirEmpreinte();
  etat.textContent = passe;
});

curseurVitesse.addEventListener("input", () => {
  sortieVitesse.textContent = String(vitesse());
});

// ---------------------------------------------------------------------------------------------
// La boucle : une image de navigateur, un ou plusieurs pas de simulation
// ---------------------------------------------------------------------------------------------

// Fige la colonie : pause, dernier dessin, mesures et empreinte, puis message. Le bouton
// Pause/Reprendre est désactivé tant qu'une nouvelle colonie n'a pas été démarrée.
function arreter(texte) {
  enPause = true;
  colonieFinie = true;
  majBoutonPause();
  boutonPause.disabled = true;
  boutonPause.setAttribute("aria-disabled", "true");
  substratAJour = false;
  dessiner();
  afficherMesures();
  rafraichirEmpreinte();
  etat.textContent = texte;
}

function boucle() {
  if (module !== null && !enPause) {
    const temps = module.mycelium_temps() >>> 0;
    // La boucle ne dépasse jamais PAS_MAX : au plus les pas qui restent avant la limite.
    module.mycelium_avancer(Math.min(vitesse(), PAS_MAX - temps));
    compteurImages += 1;
    // Le substrat diffuse lentement : le redessiner une image sur quatre suffit, et coûte moins cher.
    if (compteurImages % IMAGES_PAR_SUBSTRAT === 0) {
      substratAJour = false;
    }

    const tempsApres = module.mycelium_temps() >>> 0;
    if (module.mycelium_pointes() === 0 && tempsApres > 0) {
      // Plus aucune pointe : la colonie a fini de pousser, rien ne changerait plus.
      arreter(
        "La colonie a fini de pousser : plus aucune pointe. « Relancer » ou « Au hasard » en démarre une autre.",
      );
    } else if (tempsApres >= PAS_MAX) {
      // La limite de sécurité : on ne simule pas plus loin.
      arreter(
        `Limite de la vitrine atteinte (${PAS_MAX} pas) : la colonie est en pause. « Relancer » en démarre une autre.`,
      );
    } else {
      dessiner();
      afficherMesures();
      // Huit appels à l'empreinte, chacun hache tout l'état : environ une fois par seconde et demie.
      if (compteurImages % IMAGES_PAR_EMPREINTE === 0) {
        rafraichirEmpreinte();
      }
    }
  }
  requestAnimationFrame(boucle);
}

// ---------------------------------------------------------------------------------------------
// Les lectures : mesures, empreinte
// ---------------------------------------------------------------------------------------------

function afficherMesures() {
  if (module === null) {
    return;
  }
  champTemps.textContent = String(module.mycelium_temps() >>> 0);
  const biomasse = module.mycelium_biomasse();
  champBiomasse.textContent = Number.isFinite(biomasse) ? nombreFr.format(biomasse) : "—";
  champPointes.textContent = String(module.mycelium_pointes() >>> 0);
  champAnastomoses.textContent = String(module.mycelium_anastomoses() >>> 0);
}

function rafraichirEmpreinte() {
  if (module === null) {
    return;
  }
  let empreinte = "";
  for (let i = 0; i < 8; i += 1) {
    // La fonction rend un i32 : `>>> 0` le relit comme un entier non signé, sur 8 chiffres.
    empreinte += (module.mycelium_empreinte_mot(i) >>> 0).toString(16).padStart(8, "0");
  }
  const temps = module.mycelium_temps() >>> 0;
  const reference = module.mycelium_duree() >>> 0;
  champEmpreinte.textContent = empreinte;
  const regle =
    `Au pas de référence du scénario (pas ${reference}), la même graine redonne la même empreinte, ici ou en natif.`;
  if (scenarioCourant === "libre" && graineCourante === 1 && temps === reference) {
    noteEmpreinte.textContent =
      `Graine 1, pas ${temps} — identique à la référence du dépôt (references/couplage-graine-1.empreinte). ${regle}`;
  } else {
    noteEmpreinte.textContent =
      `Graine ${graineCourante}, pas ${temps} — le même état redonne toujours ces 64 caractères. ${regle}`;
  }
}

// ---------------------------------------------------------------------------------------------
// Le dessin
// ---------------------------------------------------------------------------------------------

// Ajuste la résolution du canvas à sa taille affichée, pour rester net sur un écran dense.
function ajusterTaille() {
  const densite = Math.min(window.devicePixelRatio || 1, 2);
  const rectangle = canvas.getBoundingClientRect();
  const largeur = Math.max(1, Math.round(rectangle.width * densite));
  const hauteur = Math.max(1, Math.round((rectangle.height || rectangle.width) * densite));
  if (canvas.width !== largeur || canvas.height !== hauteur) {
    canvas.width = largeur;
    canvas.height = hauteur;
    dessiner();
  }
}

// Prépare la couche hors écran à la taille de la grille ; rend ses dimensions.
function preparerSubstrat() {
  const largeur = module.mycelium_largeur() >>> 0;
  const hauteur = module.mycelium_hauteur() >>> 0;
  if (largeur === 0 || hauteur === 0) {
    return null;
  }
  if (
    coucheSubstrat.width !== largeur ||
    coucheSubstrat.height !== hauteur ||
    pixelsSubstrat === null
  ) {
    coucheSubstrat.width = largeur;
    coucheSubstrat.height = hauteur;
    if (contexteSubstrat === null) {
      contexteSubstrat = coucheSubstrat.getContext("2d");
    }
    pixelsSubstrat = contexteSubstrat.createImageData(largeur, hauteur);
  }
  return { largeur, hauteur };
}

// Peint le substrat dans la couche hors écran, une case par pixel.
function dessinerSubstrat() {
  const dimensions = preparerSubstrat();
  if (dimensions === null) {
    return;
  }
  const { largeur, hauteur } = dimensions;
  const pixels = pixelsSubstrat.data;
  let i = 0;
  let aUneTache = false;
  for (let y = 0; y < hauteur; y += 1) {
    for (let x = 0; x < largeur; x += 1) {
      const quantite = module.mycelium_substrat(x, y);
      const deficit = Number.isFinite(quantite) ? Math.max(0, 1 - quantite) : 1;
      const clair = Math.max(0, Math.min(1, 1 - deficit * CONTRASTE_SUBSTRAT));
      // Une case plus riche que le substrat de départ appartient à une tache de nutriments : elle se teinte.
      const excedent = Number.isFinite(quantite) ? Math.max(0, quantite - 1) : 0;
      const tache = Math.min(1, excedent / EXCEDENT_PLEIN);
      aUneTache = aUneTache || tache > 0;
      // Un grain très fin, tiré d'un petit mélange d'entiers, évite l'aplat sans dessiner de motif.
      let melange = Math.imul(x, 374761393) + Math.imul(y, 668265263);
      melange = Math.imul(melange ^ (melange >>> 13), 1274126177);
      const grain = ((melange >>> 0) % 5) - 2;
      pixels[i] = 12 + 42 * clair + 30 * tache + grain;
      pixels[i + 1] = 17 + 51 * clair + 90 * tache + grain;
      pixels[i + 2] = 16 + 28 * clair + 60 * tache;
      pixels[i + 3] = 255;
      i += 4;
    }
  }
  contexteSubstrat.putImageData(pixelsSubstrat, 0, 0);
  legendeTache.hidden = !aUneTache;
}

// Dessine les obstacles du milieu : chacun est un rectangle de cases dont les deux bords sont inclus, donc
// de x0 à x1 inclus il couvre (x1 - x0 + 1) cases, et de même en hauteur. Le liseré est tracé à l'intérieur de
// ces cases, pour que l'obstacle n'en déborde pas.
function dessinerObstacles(origineX, origineY, echelle) {
  const nombre = module.mycelium_nb_obstacles() >>> 0;
  const epaisseur = Math.max(1, echelle * 0.2);
  for (let i = 0; i < nombre; i += 1) {
    const x0 = module.mycelium_obstacle(i, 0);
    const y0 = module.mycelium_obstacle(i, 1);
    const x1 = module.mycelium_obstacle(i, 2);
    const y1 = module.mycelium_obstacle(i, 3);
    if (!Number.isFinite(x0) || !Number.isFinite(y0) || !Number.isFinite(x1) || !Number.isFinite(y1)) {
      continue;
    }
    const x = origineX + x0 * echelle;
    const y = origineY + y0 * echelle;
    const largeur = (x1 - x0 + 1) * echelle;
    const hauteur = (y1 - y0 + 1) * echelle;
    contexte.fillStyle = COULEUR_OBSTACLE;
    contexte.fillRect(x, y, largeur, hauteur);
    contexte.strokeStyle = COULEUR_LISERE_OBSTACLE;
    contexte.lineWidth = epaisseur;
    contexte.strokeRect(x + epaisseur / 2, y + epaisseur / 2, largeur - epaisseur, hauteur - epaisseur);
  }
}

// Trace tous les segments d'hyphe en un seul chemin, avec un halo doux puis un trait net.
function dessinerHyphes(origineX, origineY, echelle) {
  const nombre = module.mycelium_nb_segments() >>> 0;

  if (nombre === 0) {
    // Au temps 0, il n'y a pas encore de trait à tracer. La spore, elle, n'est pas dessinée : le module n'en
    // donne pas la position, et seul le scénario « libre » la met au centre de la boîte.
    return;
  }

  contexte.beginPath();
  for (let i = 0; i < nombre; i += 1) {
    const x0 = module.mycelium_segment(i, 0);
    const y0 = module.mycelium_segment(i, 1);
    const x1 = module.mycelium_segment(i, 2);
    const y1 = module.mycelium_segment(i, 3);
    if (!Number.isFinite(x0) || !Number.isFinite(y0) || !Number.isFinite(x1) || !Number.isFinite(y1)) {
      continue;
    }
    contexte.moveTo(origineX + x0 * echelle, origineY + y0 * echelle);
    contexte.lineTo(origineX + x1 * echelle, origineY + y1 * echelle);
  }

  contexte.lineCap = "round";
  contexte.lineJoin = "round";

  contexte.strokeStyle = "rgba(198, 228, 158, 0.15)";
  contexte.lineWidth = Math.max(2.4, echelle * 1.05);
  contexte.stroke();

  contexte.strokeStyle = "rgba(242, 247, 224, 0.93)";
  contexte.lineWidth = Math.max(0.9, echelle * 0.28);
  contexte.stroke();
}

// Trace un rectangle à coins arrondis : le bord de la boîte carrée de la simulation.
function cheminRectangleArrondi(x, y, largeur, hauteur, rayon) {
  const r = Math.max(0, Math.min(rayon, largeur / 2, hauteur / 2));
  contexte.beginPath();
  contexte.moveTo(x + r, y);
  contexte.lineTo(x + largeur - r, y);
  contexte.arcTo(x + largeur, y, x + largeur, y + r, r);
  contexte.lineTo(x + largeur, y + hauteur - r);
  contexte.arcTo(x + largeur, y + hauteur, x + largeur - r, y + hauteur, r);
  contexte.lineTo(x + r, y + hauteur);
  contexte.arcTo(x, y + hauteur, x, y + hauteur - r, r);
  contexte.lineTo(x, y + r);
  contexte.arcTo(x, y, x + r, y, r);
  contexte.closePath();
}

// Le bord de la boîte carrée et un reflet de verre, discrets.
function dessinerBord(origineX, origineY, largeurBoite, hauteurBoite, rayonCoins, cote) {
  cheminRectangleArrondi(origineX, origineY, largeurBoite, hauteurBoite, rayonCoins);
  contexte.strokeStyle = "rgba(206, 224, 200, 0.14)";
  contexte.lineWidth = Math.max(1, cote * 0.0022);
  contexte.stroke();

  // Le même contour, baigné d'un dégradé qui s'éteint vers le bas droit : un reflet de verre.
  const reflet = contexte.createLinearGradient(
    origineX, origineY,
    origineX + largeurBoite, origineY + hauteurBoite,
  );
  reflet.addColorStop(0, "rgba(232, 246, 226, 0.16)");
  reflet.addColorStop(0.45, "rgba(232, 246, 226, 0)");
  cheminRectangleArrondi(origineX, origineY, largeurBoite, hauteurBoite, rayonCoins);
  contexte.strokeStyle = reflet;
  contexte.lineWidth = Math.max(2, cote * 0.01);
  contexte.lineCap = "round";
  contexte.stroke();
}

function dessiner() {
  const largeurCanvas = canvas.width;
  const hauteurCanvas = canvas.height;

  contexte.setTransform(1, 0, 0, 1, 0, 0);
  contexte.clearRect(0, 0, largeurCanvas, hauteurCanvas);

  const fond = contexte.createRadialGradient(
    largeurCanvas * 0.5, hauteurCanvas * 0.42, 0,
    largeurCanvas * 0.5, hauteurCanvas * 0.5, largeurCanvas * 0.62,
  );
  fond.addColorStop(0, "#101a19");
  fond.addColorStop(1, "#040607");
  contexte.fillStyle = fond;
  contexte.fillRect(0, 0, largeurCanvas, hauteurCanvas);

  if (module === null) {
    return;
  }

  const largeur = module.mycelium_largeur() >>> 0;
  const hauteur = module.mycelium_hauteur() >>> 0;
  if (largeur === 0 || hauteur === 0) {
    return;
  }

  // L'échelle fait tenir toute la boîte carrée dans le canvas, avec une petite marge.
  const cote = Math.min(largeurCanvas, hauteurCanvas) * 0.94;
  const echelle = cote / Math.max(largeur, hauteur);
  const largeurBoite = largeur * echelle;
  const hauteurBoite = hauteur * echelle;
  const origineX = (largeurCanvas - largeurBoite) / 2;
  const origineY = (hauteurCanvas - hauteurBoite) / 2;
  const cx = origineX + largeurBoite / 2;
  const cy = origineY + hauteurBoite / 2;
  // Des coins à peine arrondis, pour ne pas découper le substrat.
  const rayonCoins = Math.min(largeurBoite, hauteurBoite) * 0.035;

  // Tout le contenu est tenu dans la boîte carrée, aux coins légèrement arrondis.
  contexte.save();
  cheminRectangleArrondi(origineX, origineY, largeurBoite, hauteurBoite, rayonCoins);
  contexte.clip();

  if (!substratAJour) {
    dessinerSubstrat();
    substratAJour = true;
  }
  contexte.imageSmoothingEnabled = true;
  contexte.drawImage(coucheSubstrat, origineX, origineY, largeurBoite, hauteurBoite);

  // Un vignettage qui éteint doucement les bords de la boîte carrée.
  const rayonVignette = Math.hypot(largeurBoite, hauteurBoite) / 2;
  const vignette = contexte.createRadialGradient(cx, cy, rayonVignette * 0.45, cx, cy, rayonVignette);
  vignette.addColorStop(0, "rgba(0, 0, 0, 0)");
  vignette.addColorStop(1, "rgba(0, 0, 0, 0.58)");
  contexte.fillStyle = vignette;
  contexte.fillRect(origineX, origineY, largeurBoite, hauteurBoite);

  // Les obstacles viennent après le substrat et son vignettage (leur teinte ne s'assombrit pas vers les bords),
  // et avant les hyphes, qui se tracent par-dessus.
  dessinerObstacles(origineX, origineY, echelle);
  dessinerHyphes(origineX, origineY, echelle);
  contexte.restore();

  dessinerBord(origineX, origineY, largeurBoite, hauteurBoite, rayonCoins, cote);
}

// ---------------------------------------------------------------------------------------------
// Le départ
// ---------------------------------------------------------------------------------------------

// Le menu et le champ disent d'abord ce que l'adresse demande ; la colonie démarre dessus quand le module est là.
appliquerAdresse();
ajusterTaille();
window.addEventListener("resize", ajusterTaille);
if (typeof ResizeObserver === "function") {
  new ResizeObserver(() => ajusterTaille()).observe(canvas);
}
requestAnimationFrame(boucle);
charger();
