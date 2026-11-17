// La page du cube de nourriture (étape 18 de Mycelium Lab, la première marche de la 3D).
//
// Elle charge le module WebAssembly `mycelium.wasm`, celui de la page 2D, et n'emploie que ses exportations
// `mycelium_cube_*` : le moteur calcule, la page ne fait que dessiner. Aucune ressource ne vient d'un autre site.
//
// Le volume se dessine comme un nuage : chaque case est un petit carré translucide, posé de l'arrière vers l'avant,
// d'autant plus opaque qu'il est riche ; une case de mur est un gris ardoise plus opaque. L'échelle de couleur va de
// la case libre la plus pauvre à la plus riche du moment. Le calcul se fait sur une image deux fois plus petite que
// le cadre, puis agrandie : c'est ce qui le garde rapide sur un téléphone. La coupe se dessine à plat, une case par
// carré, avec la même échelle.

const DESCRIPTIONS = [
  "Une goutte de 100 unités de nourriture, posée près du centre du cube. Elle s'étale à la même vitesse dans " +
    "toutes les directions ; les parois ne laissent rien sortir.",
  "La même goutte, posée devant un mur de deux cases d'épaisseur percé d'une fenêtre de 4 × 4 cases. " +
    "La nourriture ne passe de l'autre côté que par la fenêtre.",
];

// La rampe de couleur, du bleu nuit (peu) au jaune (beaucoup) : la même que la légende de la page.
const RAMPE = [
  [14, 24, 52],
  [36, 112, 120],
  [96, 186, 104],
  [246, 226, 92],
];
const ARDOISE = [91, 107, 115];

function couleur(t) {
  const u = Math.min(1, Math.max(0, t)) * (RAMPE.length - 1);
  const i = Math.min(RAMPE.length - 2, Math.floor(u));
  const f = u - i;
  const a = RAMPE[i];
  const b = RAMPE[i + 1];
  return [a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f, a[2] + (b[2] - a[2]) * f];
}

const etat = document.getElementById("etat");
const toileVolume = document.getElementById("volume");
const toileCoupe = document.getElementById("coupe");
const choixScene = document.getElementById("scene");
const description = document.getElementById("description");
const boutonLecture = document.getElementById("lecture");
const boutonRecommencer = document.getElementById("recommencer");
const curseurVitesse = document.getElementById("vitesse");
const vitesseLue = document.getElementById("vitesse-lue");
const curseurPosition = document.getElementById("position");
const positionLue = document.getElementById("position-lue");
const boutonsAxe = Array.from(document.querySelectorAll("[data-axe]"));
const sortiePas = document.getElementById("pas");
const sortieMasse = document.getElementById("masse");
const sortieMaximum = document.getElementById("maximum");
const sortieEmpreinte = document.getElementById("empreinte");

const vue = {
  module: null,
  dimensions: [0, 0, 0],
  valeurs: null,
  bloquees: null,
  enLecture: true,
  vitesse: 2,
  axe: 2,
  position: 16,
  lacet: -0.62,
  tangage: 0.42,
  tourneSeul: !window.matchMedia("(prefers-reduced-motion: reduce)").matches,
  tenu: null,
};

// L'image du volume, calculée à demi-résolution puis agrandie.
const DEMI = toileVolume.width / 2;
const accumulation = new Float32Array(DEMI * DEMI * 3);
const petite = document.createElement("canvas");
petite.width = DEMI;
petite.height = DEMI;
const ctxPetite = petite.getContext("2d");
const imagePetite = ctxPetite.createImageData(DEMI, DEMI);
const ctxVolume = toileVolume.getContext("2d");
const ctxCoupe = toileCoupe.getContext("2d");

function indice(x, y, z) {
  const [l, h] = vue.dimensions;
  return (z * h + y) * l + x;
}

// Lit tout le champ dans un tableau, une case par appel au module (32 768 appels pour le cube de 32).
function lireChamp() {
  const m = vue.module;
  const [l, h, p] = vue.dimensions;
  for (let z = 0; z < p; z++) {
    for (let y = 0; y < h; y++) {
      for (let x = 0; x < l; x++) {
        vue.valeurs[indice(x, y, z)] = m.mycelium_cube_valeur(x, y, z);
      }
    }
  }
}

function demarrer(pasDeDepart = 0) {
  const m = vue.module;
  m.mycelium_cube_choisir(Number(choixScene.value));
  m.mycelium_cube_demarrer();
  vue.dimensions = [m.mycelium_cube_dimension(0), m.mycelium_cube_dimension(1), m.mycelium_cube_dimension(2)];
  const [l, h, p] = vue.dimensions;
  vue.valeurs = new Float64Array(l * h * p);
  vue.bloquees = new Uint8Array(l * h * p);
  for (let z = 0; z < p; z++) {
    for (let y = 0; y < h; y++) {
      for (let x = 0; x < l; x++) {
        vue.bloquees[indice(x, y, z)] = m.mycelium_cube_bloquee(x, y, z);
      }
    }
  }
  description.textContent = DESCRIPTIONS[Number(choixScene.value)];
  // La scène du mur s'ouvre sur la coupe de côté (axe x), celle qui montre le mur et sa fenêtre.
  choisirAxe(Number(choixScene.value) === 1 ? 0 : 2);
  vue.position = Math.floor(vue.dimensions[vue.axe] / 2);
  regleCurseurPosition();
  vue.enLecture = true;
  if (pasDeDepart > 0) m.mycelium_cube_avancer(Math.min(pasDeDepart, m.mycelium_cube_duree()));
  lireChamp();
  mettreAJourMesures();
}

function choisirAxe(axe) {
  vue.axe = axe;
  for (const bouton of boutonsAxe) bouton.setAttribute("aria-pressed", String(Number(bouton.dataset.axe) === axe));
}

function regleCurseurPosition() {
  const max = vue.dimensions[vue.axe] - 1;
  curseurPosition.max = String(max);
  vue.position = Math.min(vue.position, max);
  curseurPosition.value = String(vue.position);
  positionLue.textContent = String(vue.position);
}

// La projection : le centre du cube à l'origine, une rotation (lacet autour de la verticale, puis tangage), puis
// une perspective douce. Rend [x, y, échelle] en pixels de l'image entière.
function projeter(x, y, z) {
  const [l, h, p] = vue.dimensions;
  const taille = Math.max(l, h, p);
  const cx = x - l / 2;
  const cy = y - h / 2;
  const cz = z - p / 2;
  const cl = Math.cos(vue.lacet);
  const sl = Math.sin(vue.lacet);
  const ct = Math.cos(vue.tangage);
  const st = Math.sin(vue.tangage);
  const x1 = cx * cl + cz * sl;
  const z1 = -cx * sl + cz * cl;
  const y2 = cy * ct - z1 * st;
  const z2 = cy * st + z1 * ct;
  const distance = taille * 2.6;
  const focale = toileVolume.width * 1.3;
  const echelle = focale / (distance + z2);
  return [toileVolume.width / 2 + x1 * echelle, toileVolume.height / 2 + y2 * echelle, echelle];
}

// Le plus petit et le plus grand champ des cases libres : l'échelle de couleur va de l'un à l'autre, pour que l'écart
// se voie même quand la nourriture a rempli tout le cube.
function bornesDuChamp() {
  let bas = Infinity;
  let haut = 0;
  for (let i = 0; i < vue.valeurs.length; i++) {
    if (vue.bloquees[i]) continue;
    const v = vue.valeurs[i];
    if (v < bas) bas = v;
    if (v > haut) haut = v;
  }
  return bas < haut ? [bas, 1 / (haut - bas)] : [0, haut > 0 ? 1 / haut : 0];
}

function dessinerVolume() {
  const [l, h, p] = vue.dimensions;
  for (let k = 0; k < accumulation.length; k += 3) {
    accumulation[k] = 4;
    accumulation[k + 1] = 6;
    accumulation[k + 2] = 7;
  }
  const [bas, inverse] = bornesDuChamp();
  // De l'arrière vers l'avant : chaque axe se parcourt dans le sens où la profondeur décroît.
  const ct = Math.cos(vue.tangage);
  const sens = (pente, n) => (pente > 0 ? [n - 1, -1, -1] : [0, n, 1]);
  const [z0, z1, dz] = sens(Math.cos(vue.lacet) * ct, p);
  const [y0, y1, dy] = sens(Math.sin(vue.tangage), h);
  const [x0, x1, dx] = sens(-Math.sin(vue.lacet) * ct, l);
  for (let z = z0; z !== z1; z += dz) {
    for (let y = y0; y !== y1; y += dy) {
      for (let x = x0; x !== x1; x += dx) {
        const i = indice(x, y, z);
        let rgb;
        let opacite;
        if (vue.bloquees[i]) {
          rgb = ARDOISE;
          opacite = 0.6;
        } else {
          const t = Math.sqrt(Math.max(0, vue.valeurs[i] - bas) * inverse);
          opacite = 0.012 + 0.13 * t * t;
          if (opacite < 0.02) continue;
          rgb = couleur(t);
        }
        const [px, py, echelle] = projeter(x + 0.5, y + 0.5, z + 0.5);
        const rayon = Math.max(1, Math.round(echelle * 0.3));
        const ux = Math.round(px / 2);
        const uy = Math.round(py / 2);
        const reste = 1 - opacite;
        const [r, g, b] = [rgb[0] * opacite, rgb[1] * opacite, rgb[2] * opacite];
        for (let ey = -rayon; ey < rayon; ey++) {
          const ly = uy + ey;
          if (ly < 0 || ly >= DEMI) continue;
          for (let ex = -rayon; ex < rayon; ex++) {
            const lx = ux + ex;
            if (lx < 0 || lx >= DEMI) continue;
            const k = (ly * DEMI + lx) * 3;
            accumulation[k] = accumulation[k] * reste + r;
            accumulation[k + 1] = accumulation[k + 1] * reste + g;
            accumulation[k + 2] = accumulation[k + 2] * reste + b;
          }
        }
      }
    }
  }
  const pixels = imagePetite.data;
  for (let k = 0, j = 0; k < accumulation.length; k += 3, j += 4) {
    pixels[j] = accumulation[k];
    pixels[j + 1] = accumulation[k + 1];
    pixels[j + 2] = accumulation[k + 2];
    pixels[j + 3] = 255;
  }
  ctxPetite.putImageData(imagePetite, 0, 0);
  ctxVolume.imageSmoothingEnabled = true;
  ctxVolume.drawImage(petite, 0, 0, toileVolume.width, toileVolume.height);
  dessinerAretes();
  dessinerPlanDeCoupe();
}

function trait(a, b) {
  ctxVolume.moveTo(a[0], a[1]);
  ctxVolume.lineTo(b[0], b[1]);
}

function dessinerAretes() {
  const [l, h, p] = vue.dimensions;
  const coins = [];
  for (const z of [0, p]) for (const y of [0, h]) for (const x of [0, l]) coins.push(projeter(x, y, z));
  ctxVolume.save();
  ctxVolume.lineWidth = 1.5;
  ctxVolume.strokeStyle = "rgba(205, 226, 203, 0.38)";
  ctxVolume.beginPath();
  for (let i = 0; i < 8; i++) {
    for (const bit of [1, 2, 4]) {
      if (!(i & bit)) trait(coins[i], coins[i | bit]);
    }
  }
  ctxVolume.stroke();
  ctxVolume.fillStyle = "rgba(233, 239, 230, 0.75)";
  ctxVolume.font = "600 22px ui-sans-serif, system-ui, sans-serif";
  const etiquettes = [
    ["x", projeter(l + 2.5, 0, 0)],
    ["y", projeter(0, h + 2.5, 0)],
    ["z", projeter(0, 0, p + 2.5)],
  ];
  for (const [nom, [px, py]] of etiquettes) ctxVolume.fillText(nom, px - 6, py + 7);
  ctxVolume.restore();
}

function coinsDeLaCoupe() {
  const [l, h, p] = vue.dimensions;
  const c = vue.position + 0.5;
  if (vue.axe === 0) return [[c, 0, 0], [c, h, 0], [c, h, p], [c, 0, p]];
  if (vue.axe === 1) return [[0, c, 0], [l, c, 0], [l, c, p], [0, c, p]];
  return [[0, 0, c], [l, 0, c], [l, h, c], [0, h, c]];
}

function dessinerPlanDeCoupe() {
  const coins = coinsDeLaCoupe().map(([x, y, z]) => projeter(x, y, z));
  ctxVolume.save();
  ctxVolume.beginPath();
  ctxVolume.moveTo(coins[0][0], coins[0][1]);
  for (const coin of coins.slice(1)) ctxVolume.lineTo(coin[0], coin[1]);
  ctxVolume.closePath();
  ctxVolume.fillStyle = "rgba(207, 230, 166, 0.06)";
  ctxVolume.fill();
  ctxVolume.lineWidth = 2;
  ctxVolume.strokeStyle = "rgba(207, 230, 166, 0.7)";
  ctxVolume.setLineDash([10, 7]);
  ctxVolume.stroke();
  ctxVolume.restore();
}

// La coupe : la tranche de l'axe choisi, à plat. Axe z : x en largeur, y en hauteur ; axe y : x en largeur, z en
// hauteur ; axe x : y en largeur, z en hauteur.
function dessinerCoupe() {
  const [l, h, p] = vue.dimensions;
  const [largeur, hauteur] = vue.axe === 0 ? [h, p] : vue.axe === 1 ? [l, p] : [l, h];
  const image = ctxCoupe.createImageData(largeur, hauteur);
  const [bas, inverse] = bornesDuChamp();
  for (let v = 0; v < hauteur; v++) {
    for (let u = 0; u < largeur; u++) {
      const [x, y, z] =
        vue.axe === 0 ? [vue.position, u, v] : vue.axe === 1 ? [u, vue.position, v] : [u, v, vue.position];
      const i = indice(x, y, z);
      const rgb = vue.bloquees[i] ? ARDOISE : couleur(Math.sqrt(Math.max(0, vue.valeurs[i] - bas) * inverse));
      const j = (v * largeur + u) * 4;
      image.data[j] = rgb[0];
      image.data[j + 1] = rgb[1];
      image.data[j + 2] = rgb[2];
      image.data[j + 3] = 255;
    }
  }
  const tampon = document.createElement("canvas");
  tampon.width = largeur;
  tampon.height = hauteur;
  tampon.getContext("2d").putImageData(image, 0, 0);
  ctxCoupe.imageSmoothingEnabled = false;
  ctxCoupe.clearRect(0, 0, toileCoupe.width, toileCoupe.height);
  ctxCoupe.drawImage(tampon, 0, 0, toileCoupe.width, toileCoupe.height);
}

function empreinte() {
  let texte = "";
  for (let i = 0; i < 8; i++) texte += (vue.module.mycelium_cube_empreinte_mot(i) >>> 0).toString(16).padStart(8, "0");
  return texte;
}

function mettreAJourMesures() {
  const m = vue.module;
  const temps = m.mycelium_cube_temps();
  const duree = m.mycelium_cube_duree();
  sortiePas.textContent = `${temps} / ${duree}`;
  sortieMasse.textContent = m.mycelium_cube_masse().toFixed(6);
  sortieMaximum.textContent = m.mycelium_cube_maximum().toExponential(3);
  const e = empreinte();
  sortieEmpreinte.textContent = `${e.slice(0, 10)}…`;
  sortieEmpreinte.title = e;
  if (temps >= duree) {
    etat.textContent = `Fin de la scène, au pas ${duree}. L'empreinte complète est dans la bulle de la mesure.`;
  } else if (vue.enLecture) {
    etat.textContent = "La nourriture diffuse. Glissez sur le cube pour le tourner.";
  } else {
    etat.textContent = "En pause.";
  }
  boutonLecture.textContent = vue.enLecture && temps < duree ? "Pause" : "Lecture";
}

function image() {
  const m = vue.module;
  const temps = m.mycelium_cube_temps();
  const duree = m.mycelium_cube_duree();
  if (vue.enLecture && temps < duree) {
    m.mycelium_cube_avancer(Math.min(vue.vitesse, duree - temps));
    lireChamp();
    mettreAJourMesures();
  } else if (vue.enLecture) {
    vue.enLecture = false;
    mettreAJourMesures();
  }
  if (vue.tourneSeul && vue.tenu === null) vue.lacet += 0.0035;
  dessinerVolume();
  dessinerCoupe();
  requestAnimationFrame(image);
}

toileVolume.addEventListener("pointerdown", (evenement) => {
  vue.tenu = [evenement.clientX, evenement.clientY];
  vue.tourneSeul = false;
  toileVolume.setPointerCapture(evenement.pointerId);
  toileVolume.classList.add("tenu");
});
toileVolume.addEventListener("pointermove", (evenement) => {
  if (vue.tenu === null) return;
  const [x0, y0] = vue.tenu;
  vue.lacet += (evenement.clientX - x0) * 0.008;
  vue.tangage = Math.max(-1.45, Math.min(1.45, vue.tangage + (evenement.clientY - y0) * 0.008));
  vue.tenu = [evenement.clientX, evenement.clientY];
});
for (const fin of ["pointerup", "pointercancel"]) {
  toileVolume.addEventListener(fin, () => {
    vue.tenu = null;
    toileVolume.classList.remove("tenu");
  });
}

boutonLecture.addEventListener("click", () => {
  const m = vue.module;
  if (m.mycelium_cube_temps() >= m.mycelium_cube_duree()) demarrer();
  else vue.enLecture = !vue.enLecture;
  mettreAJourMesures();
});
// L'adresse peut choisir la scène et le pas de départ : `cube.html?scene=1&pas=400` ouvre la fin du mur.
const adresse = new URLSearchParams(window.location.search);
const sceneDemandee = Number(adresse.get("scene"));
if (sceneDemandee === 0 || sceneDemandee === 1) choixScene.value = String(sceneDemandee);
const pasDemande = Math.max(0, Math.floor(Number(adresse.get("pas")) || 0));
boutonRecommencer.addEventListener("click", () => demarrer());
choixScene.addEventListener("change", () => demarrer());
curseurVitesse.addEventListener("input", () => {
  vue.vitesse = Number(curseurVitesse.value);
  vitesseLue.textContent = vue.vitesse === 1 ? "1 pas par image" : `${vue.vitesse} pas par image`;
});
curseurPosition.addEventListener("input", () => {
  vue.position = Number(curseurPosition.value);
  positionLue.textContent = String(vue.position);
});
for (const bouton of boutonsAxe) {
  bouton.addEventListener("click", () => {
    choisirAxe(Number(bouton.dataset.axe));
    regleCurseurPosition();
  });
}

async function charger() {
  try {
    const reponse = await fetch("mycelium.wasm");
    if (!reponse.ok) throw new Error(`le module répond ${reponse.status}`);
    const { instance } = await WebAssembly.instantiate(await reponse.arrayBuffer(), {});
    vue.module = instance.exports;
    demarrer(pasDemande);
    requestAnimationFrame(image);
  } catch (erreur) {
    etat.textContent = `Le moteur n'a pas pu se charger : ${erreur.message}.`;
  }
}

charger();
