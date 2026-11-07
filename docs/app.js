/*
  La vitrine de Mycelium Lab.

  La page charge le module WebAssembly `mycelium.wasm`, démarre la simulation avec une graine,
  puis l'avance d'un certain nombre de pas à chaque image et la dessine sur le canvas. Le module
  n'importe rien : toutes les fonctions qu'il exporte ne prennent et ne rendent que des nombres.

  Aucune ressource externe : pas de police, pas d'image, pas de bibliothèque. Le rendu est fait
  avec le contexte 2D du canvas ; la simulation est le module lui-même.

  L'empreinte de l'état est chère à lire (huit appels, chacun hache tout l'état), donc on ne la
  rafraîchit qu'à intervalles espacés, et toujours en pause, au redémarrage ou au saut au pas 80.
*/

const GRAINE_MAX = 4294967295;
const PAS_REFERENCE = 80;

// Le substrat n'est consommé qu'à quelques millièmes par pas : on étale cet écart pour que la
// déplétion se voie, sans jamais modifier les valeurs lues dans le module.
const CONTRASTE_SUBSTRAT = 60;

// Périodes de rafraîchissement, en images affichées (et non en pas de simulation).
const IMAGES_PAR_SUBSTRAT = 4;
const IMAGES_PAR_EMPREINTE = 90;

// Les éléments de la page, tous désignés par leur identifiant.
const canvas = document.getElementById("boite");
const contexte = canvas.getContext("2d");

const champGraine = document.getElementById("graine");
const boutonHasard = document.getElementById("hasard");
const boutonRelancer = document.getElementById("relancer");
const boutonPause = document.getElementById("pause");
const boutonPas80 = document.getElementById("pas80");
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
let graineCourante = 1;
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
    demarrer(1);
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
  const texte = champGraine.value.trim();
  if (!/^[0-9]+$/.test(texte)) {
    return null;
  }
  const valeur = Number(texte);
  if (!Number.isSafeInteger(valeur) || valeur > GRAINE_MAX) {
    return null;
  }
  return valeur;
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

// Démarre une nouvelle simulation avec cette graine, en repartant du temps 0.
function demarrer(graine) {
  graineCourante = graine;
  champGraine.value = String(graine);
  module.mycelium_demarrer(graine);
  enPause = false;
  compteurImages = 0;
  substratAJour = false;
  majBoutonPause();
  dessiner();
  afficherMesures();
  rafraichirEmpreinte();
  etat.textContent = "La colonie pousse.";
  message.textContent = "";
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
  demarrer(graine);
});

boutonHasard.addEventListener("click", () => {
  if (!pret()) {
    return;
  }
  demarrer(graineAleatoire());
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

boutonPas80.addEventListener("click", () => {
  if (!pret()) {
    return;
  }
  const temps = module.mycelium_temps() >>> 0;
  if (temps >= PAS_REFERENCE) {
    etat.textContent = `La colonie est déjà au pas ${temps}.`;
    return;
  }
  module.mycelium_avancer(PAS_REFERENCE - temps);
  // On s'arrête au pas de la référence : l'empreinte affichée reste celle du dépôt.
  enPause = true;
  majBoutonPause();
  substratAJour = false;
  dessiner();
  afficherMesures();
  rafraichirEmpreinte();
  etat.textContent = `Pas ${PAS_REFERENCE} atteint : la colonie est en pause.`;
});

curseurVitesse.addEventListener("input", () => {
  sortieVitesse.textContent = String(vitesse());
});

// ---------------------------------------------------------------------------------------------
// La boucle : une image de navigateur, un ou plusieurs pas de simulation
// ---------------------------------------------------------------------------------------------

function boucle() {
  if (module !== null && !enPause) {
    module.mycelium_avancer(vitesse());
    compteurImages += 1;
    // Le substrat diffuse lentement : le redessiner une image sur quatre suffit, et coûte moins cher.
    if (compteurImages % IMAGES_PAR_SUBSTRAT === 0) {
      substratAJour = false;
    }
    dessiner();
    afficherMesures();
    // Huit appels à l'empreinte, chacun hache tout l'état : environ une fois par seconde et demie.
    if (compteurImages % IMAGES_PAR_EMPREINTE === 0) {
      rafraichirEmpreinte();
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
  champEmpreinte.textContent = empreinte;
  if (graineCourante === 1 && temps === PAS_REFERENCE) {
    noteEmpreinte.textContent =
      "Graine 1, pas 80 — identique à la référence du dépôt (references/couplage-graine-1.empreinte).";
  } else {
    noteEmpreinte.textContent =
      `Graine ${graineCourante}, pas ${temps} — le même état redonne toujours ces 64 caractères.`;
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
  if (coucheSubstrat.width !== largeur || coucheSubstrat.height !== hauteur) {
    coucheSubstrat.width = largeur;
    coucheSubstrat.height = hauteur;
    contexteSubstrat = coucheSubstrat.getContext("2d");
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
  for (let y = 0; y < hauteur; y += 1) {
    for (let x = 0; x < largeur; x += 1) {
      const quantite = module.mycelium_substrat(x, y);
      const deficit = Number.isFinite(quantite) ? Math.max(0, 1 - quantite) : 1;
      const clair = Math.max(0, Math.min(1, 1 - deficit * CONTRASTE_SUBSTRAT));
      // Un grain très fin, tiré d'un petit mélange d'entiers, évite l'aplat sans dessiner de motif.
      let melange = Math.imul(x, 374761393) + Math.imul(y, 668265263);
      melange = Math.imul(melange ^ (melange >>> 13), 1274126177);
      const grain = ((melange >>> 0) % 5) - 2;
      pixels[i] = 12 + 42 * clair + grain;
      pixels[i + 1] = 17 + 51 * clair + grain;
      pixels[i + 2] = 16 + 28 * clair;
      pixels[i + 3] = 255;
      i += 4;
    }
  }
  contexteSubstrat.putImageData(pixelsSubstrat, 0, 0);
}

// Trace tous les segments d'hyphe en un seul chemin, avec un halo doux puis un trait net.
function dessinerHyphes(origineX, origineY, echelle) {
  const nombre = module.mycelium_nb_segments() >>> 0;

  if (nombre === 0) {
    // Au temps 0, la spore : un point lumineux au centre de la boîte.
    const x = origineX + ((module.mycelium_largeur() >>> 0) / 2) * echelle;
    const y = origineY + ((module.mycelium_hauteur() >>> 0) / 2) * echelle;
    const halo = contexte.createRadialGradient(x, y, 0, x, y, echelle * 5);
    halo.addColorStop(0, "rgba(236, 250, 205, 0.9)");
    halo.addColorStop(0.4, "rgba(190, 222, 150, 0.24)");
    halo.addColorStop(1, "rgba(190, 222, 150, 0)");
    contexte.fillStyle = halo;
    contexte.beginPath();
    contexte.arc(x, y, echelle * 5, 0, Math.PI * 2);
    contexte.fill();
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

// Le bord de la boîte de Petri et un reflet de verre, discrets.
function dessinerBord(cx, cy, rayon, cote) {
  contexte.beginPath();
  contexte.arc(cx, cy, rayon, 0, Math.PI * 2);
  contexte.strokeStyle = "rgba(206, 224, 200, 0.14)";
  contexte.lineWidth = Math.max(1, cote * 0.0022);
  contexte.stroke();

  contexte.beginPath();
  contexte.arc(cx, cy, rayon * 0.99, Math.PI * 1.04, Math.PI * 1.4);
  contexte.strokeStyle = "rgba(232, 246, 226, 0.12)";
  contexte.lineWidth = rayon * 0.018;
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

  const cote = Math.min(largeurCanvas, hauteurCanvas) * 0.94;
  const rayon = cote / 2;
  const cx = largeurCanvas / 2;
  const cy = hauteurCanvas / 2;
  const echelle = cote / Math.max(largeur, hauteur);
  const origineX = cx - (largeur * echelle) / 2;
  const origineY = cy - (hauteur * echelle) / 2;

  // Tout le contenu est tenu dans le disque de la boîte.
  contexte.save();
  contexte.beginPath();
  contexte.arc(cx, cy, rayon, 0, Math.PI * 2);
  contexte.clip();

  if (!substratAJour) {
    dessinerSubstrat();
    substratAJour = true;
  }
  contexte.imageSmoothingEnabled = true;
  contexte.drawImage(coucheSubstrat, origineX, origineY, largeur * echelle, hauteur * echelle);

  // Un vignettage qui éteint doucement les bords de la boîte.
  const vignette = contexte.createRadialGradient(cx, cy, rayon * 0.5, cx, cy, rayon);
  vignette.addColorStop(0, "rgba(0, 0, 0, 0)");
  vignette.addColorStop(1, "rgba(0, 0, 0, 0.58)");
  contexte.fillStyle = vignette;
  contexte.fillRect(0, 0, largeurCanvas, hauteurCanvas);

  dessinerHyphes(origineX, origineY, echelle);
  contexte.restore();

  dessinerBord(cx, cy, rayon, largeurCanvas);
}

// ---------------------------------------------------------------------------------------------
// Le départ
// ---------------------------------------------------------------------------------------------

ajusterTaille();
window.addEventListener("resize", ajusterTaille);
if (typeof ResizeObserver === "function") {
  new ResizeObserver(() => ajusterTaille()).observe(canvas);
}
requestAnimationFrame(boucle);
charger();
