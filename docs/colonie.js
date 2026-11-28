// La page de la colonie dans le cube (étapes 21 et 25 de Mycelium Lab, la deuxième marche de la 3D).
//
// Elle charge le module WebAssembly `mycelium.wasm`, celui des autres pages, et n'emploie que ses exportations
// `mycelium_colonie_*` : le moteur calcule, la page ne fait que dessiner. Le dessin passe par three.js, copié dans
// `bibliotheques/three/` : aucune ressource ne vient d'un autre site.
//
// La colonie est un seul objet `LineSegments`, dont les tampons de positions et de couleurs sont réservés une fois
// par démarrage, à la taille finale de la scène, puis remplis au fil des pas : seuls les nouveaux segments sont
// écrits, et `setDrawRange` montre ce qui est rempli. Le dernier pas joué pousse en douceur : ses segments
// s'allongent de 0 à leur longueur pendant le temps d'un pas, puis restent tels que le moteur les a laissés. Les
// soudures (le lieu de chaque anastomose, lu dans le module) sont un objet `Points` tenu de la même façon : celles du
// dernier pas paraissent quand ses segments ont fini de s'allonger. Les obstacles du scénario sont des pavés
// translucides, posés à chaque démarrage.
//
// Les axes : le moteur compte en cases, de 0 à la largeur (x), la hauteur (y) et la profondeur (z). Le cube est
// centré sur l'origine, et la verticale de three.js est le z du moteur : X = x − L/2, Y = z − P/2, Z = H/2 − y.
// C'est une rotation (d'un quart de tour autour de x), pas un miroir : la colonie n'est pas inversée.

import * as THREE from "./bibliotheques/three/three.module.min.js";

// Une description par scène, dans l'ordre des numéros du module (0 à 4).
const DESCRIPTIONS = [
  "La spore au centre d'un cube où la nourriture est partout la même. Rien n'oriente les filaments : ils marchent " +
    "au hasard et se ramifient. Partie d'une seule pointe, la colonie s'ouvre en éventail du côté où celle-ci est " +
    "partie, et atteint la paroi avant de s'ouvrir tout à fait.",
  "Peu de nourriture partout, et une goutte de 3 000 unités à 10 cases de la spore. Un filament avance d'autant " +
    "moins qu'il a peu à manger : la colonie s'étend du côté de la goutte.",
  "La même goutte, mais les pointes la sentent : à chaque pas, une pointe se penche d'au plus 10 degrés vers le " +
    "côté où il y a plus de nourriture (le chimiotropisme). La colonie file vers la goutte et s'y nourrit : avec la " +
    "graine 1, elle laisse quatre fois plus de filaments que sans.",
  "Un mur de deux cases d'épaisseur coupe le cube, percé en son centre d'une fenêtre de 4 × 4 cases. La spore est " +
    "d'un côté, une goutte de l'autre, qui ne diffuse que par la fenêtre. Une pointe qui touche le mur sous moins de " +
    "35 degrés le longe ; plus de face, elle s'arrête et jusqu'à deux pointes repartent le long du mur. Le passage " +
    "dépend beaucoup de la graine : avec la 1, 7 % des filaments finissent de l'autre côté ; avec la 3, aucun.",
  "La nourriture partout, et une règle de plus : une pointe qui arrive à une demi-case au plus d'un filament laissé " +
    "par une autre pointe il y a 10 pas ou plus s'y soude et cesse de pousser (l'anastomose). En volume, deux " +
    "filaments se croisent rarement : avec la graine 1, la première soudure vient au pas 43 sur 50.",
];

// Les empreintes de la fin de chaque scène avec la graine 1, celles du moteur natif (`references/colonie-*.empreinte`),
// dans l'ordre des scènes.
const REFERENCES = [
  "2dc3370c51912df291e0f96fbd9dadc7a5b2d04719d6a51f909c768ebf935440",
  "4d9a09776ecd70f555878e4284e7e7431bbccde24a1ec104ec54657cd4555e51",
  "1c09aab224ddb44605c0bb297ea200d337f217d35a2e90e54c94cae24aee6cc4",
  "2cdee7029ee7d132ebbac4dad5a495f93110e1f5f663625c6846e5d48fbbca80",
  "eea0dcebb30da120d56dbb18fa2fda4d3cd412d3d9a17c6f8cf67bfa04f72f09",
];
const GRAINE_DE_REFERENCE = 1;
const GRAINE_MAX = 4294967295;

// La rampe de couleur du temps de naissance, du bleu profond (le premier pas) au jaune-vert (le dernier). Le bleu
// est plus clair que celui de cube.js : un filament doit se voir sur le fond sombre.
const RAMPE = [
  [44, 78, 190],
  [40, 150, 190],
  [110, 210, 120],
  [228, 246, 110],
];

const teinte = new THREE.Color();

function couleur(t) {
  const u = Math.min(1, Math.max(0, t)) * (RAMPE.length - 1);
  const i = Math.min(RAMPE.length - 2, Math.floor(u));
  const f = u - i;
  const a = RAMPE[i];
  const b = RAMPE[i + 1];
  // three.js mélange les couleurs en linéaire : la rampe, écrite en sRGB comme en CSS, y est convertie.
  teinte.setRGB(
    (a[0] + (b[0] - a[0]) * f) / 255,
    (a[1] + (b[1] - a[1]) * f) / 255,
    (a[2] + (b[2] - a[2]) * f) / 255,
    THREE.SRGBColorSpace,
  );
  return [teinte.r, teinte.g, teinte.b];
}

const etat = document.getElementById("etat");
const toile = document.getElementById("volume");
const choixScene = document.getElementById("scene");
const description = document.getElementById("description");
const boutonLecture = document.getElementById("lecture");
const boutonRecommencer = document.getElementById("recommencer");
const curseurVitesse = document.getElementById("vitesse");
const vitesseLue = document.getElementById("vitesse-lue");
const champGraine = document.getElementById("graine");
const legendeGoutte = document.getElementById("legende-goutte");
const legendeMur = document.getElementById("legende-mur");
const legendeSoudure = document.getElementById("legende-soudure");
const sortiePas = document.getElementById("pas");
const sortieFilaments = document.getElementById("filaments");
const sortiePointes = document.getElementById("pointes");
const sortieRamifications = document.getElementById("ramifications");
const sortieSoudures = document.getElementById("soudures");
const sortieMasse = document.getElementById("masse");
const sortieEmpreinte = document.getElementById("empreinte");
const verdict = document.getElementById("verdict");

const vue = {
  module: null,
  dimensions: [32, 32, 32],
  graine: GRAINE_DE_REFERENCE,
  enLecture: true,
  cadence: 6,
  // La part du pas en cours déjà dessinée, de 0 à 1 ; le pas suivant se joue quand elle atteint 1.
  avance: 0,
  avant: null,
  lacet: -0.62,
  tangage: 0.38,
  distance: 88,
  tourneSeul: !window.matchMedia("(prefers-reduced-motion: reduce)").matches,
  pointeurs: new Map(),
  ecart: 0,
  // Les segments déjà écrits dans les tampons, et le premier segment du dernier pas joué.
  ecrits: 0,
  debutDernierPas: 0,
  // Les arrivées vraies des segments du dernier pas, pour les allonger sans relire le module.
  arrivees: new Float32Array(0),
  // La part déjà montrée des segments du dernier pas (elle ne fait que croître), et la dernière part écrite.
  croissance: 1,
  allonge: -1,
  // Les soudures déjà écrites, et celles d'avant le dernier pas (les seules montrées tant qu'il s'allonge).
  souduresEcrites: 0,
  souduresAvantDernierPas: 0,
  empreinteLue: false,
  // La taille du dessin en pixels CSS et le rapport de pixels, pour ne redimensionner qu'au besoin.
  taille: 0,
  ratio: 0,
};

// La scène three.js : le fond, la caméra, les arêtes du cube, le mur, la colonie, ses soudures, la goutte.
let rendu;
try {
  rendu = new THREE.WebGLRenderer({ canvas: toile, antialias: true });
} catch (erreur) {
  rendu = null;
}
const scene = new THREE.Scene();
// Un brouillard léger, de la couleur du fond : le loin s'assombrit, et l'œil lit la profondeur.
scene.fog = new THREE.Fog(0x040607, 60, 130);
const camera = new THREE.PerspectiveCamera(35, 1, 0.5, 600);
const aretes = new THREE.LineSegments(
  new THREE.BufferGeometry(),
  new THREE.LineBasicMaterial({ color: 0xcde2cb, transparent: true, opacity: 0.3 }),
);
scene.add(aretes);
// Les obstacles : un pavé translucide par ligne `obstacle` du scénario, et ses arêtes plus nettes.
const mur = new THREE.Group();
const matiereMur = new THREE.MeshBasicMaterial({
  color: 0x6f8796,
  transparent: true,
  opacity: 0.2,
  depthWrite: false,
  side: THREE.DoubleSide,
});
const matiereAretesMur = new THREE.LineBasicMaterial({ color: 0xa9bfcc, transparent: true, opacity: 0.6 });
scene.add(mur);
const colonie = new THREE.LineSegments(
  new THREE.BufferGeometry(),
  new THREE.LineBasicMaterial({ vertexColors: true, transparent: true, opacity: 0.92, depthWrite: false }),
);
colonie.frustumCulled = false;
scene.add(colonie);
const soudures = new THREE.Points(
  new THREE.BufferGeometry(),
  new THREE.PointsMaterial({
    map: texteRonde(),
    color: 0xff3dd6,
    size: 2.4,
    sizeAttenuation: true,
    transparent: true,
    // Dessinées par-dessus les filaments, sans mélange additif : sur une colonie dense et claire, une soudure doit
    // rester magenta.
    depthTest: false,
    depthWrite: false,
  }),
);
soudures.frustumCulled = false;
soudures.renderOrder = 2;
scene.add(soudures);
const goutte = new THREE.Sprite(
  new THREE.SpriteMaterial({
    map: texteRonde(),
    color: 0xf2a65a,
    transparent: true,
    depthWrite: false,
    blending: THREE.AdditiveBlending,
  }),
);
goutte.scale.set(4.5, 4.5, 1);
goutte.visible = false;
scene.add(goutte);
const etiquettes = ["x", "y", "z"].map((nom) => {
  const sprite = new THREE.Sprite(new THREE.SpriteMaterial({ map: texteLettre(nom), transparent: true }));
  sprite.scale.set(2.4, 2.4, 1);
  scene.add(sprite);
  return sprite;
});

// Une tache ronde et floue, pour la goutte et les soudures.
function texteRonde() {
  const c = document.createElement("canvas");
  c.width = c.height = 64;
  const ctx = c.getContext("2d");
  const degrade = ctx.createRadialGradient(32, 32, 0, 32, 32, 32);
  degrade.addColorStop(0, "rgba(255, 255, 255, 1)");
  degrade.addColorStop(0.35, "rgba(255, 255, 255, 0.55)");
  degrade.addColorStop(1, "rgba(255, 255, 255, 0)");
  ctx.fillStyle = degrade;
  ctx.fillRect(0, 0, 64, 64);
  return new THREE.CanvasTexture(c);
}

// Une lettre claire sur fond transparent, pour nommer un axe.
function texteLettre(nom) {
  const c = document.createElement("canvas");
  c.width = c.height = 64;
  const ctx = c.getContext("2d");
  ctx.fillStyle = "rgba(233, 239, 230, 0.75)";
  ctx.font = "600 44px ui-sans-serif, system-ui, sans-serif";
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  ctx.fillText(nom, 32, 34);
  const texture = new THREE.CanvasTexture(c);
  texture.colorSpace = THREE.SRGBColorSpace;
  return texture;
}

// Le passage des cases du moteur à l'espace de three.js (voir l'en-tête du fichier).
function versScene(x, y, z, sortie, j) {
  const [l, h, p] = vue.dimensions;
  sortie[j] = x - l / 2;
  sortie[j + 1] = z - p / 2;
  sortie[j + 2] = h / 2 - y;
}

function poserAretes() {
  const [l, h, p] = vue.dimensions;
  aretes.geometry.dispose();
  aretes.geometry = new THREE.EdgesGeometry(new THREE.BoxGeometry(l, p, h));
  const point = new Float32Array(3);
  const reperes = [
    [l + 2.5, 0, 0],
    [0, h + 2.5, 0],
    [0, 0, p + 2.5],
  ];
  reperes.forEach(([x, y, z], k) => {
    versScene(x, y, z, point, 0);
    etiquettes[k].position.set(point[0], point[1], point[2]);
  });
}

// Les obstacles du scénario : chaque pavé va de la case (x0, y0, z0) à la case (x1, y1, z1), bornes comprises, donc
// de x0 à x1 + 1 en cases, et de même en y et z.
function poserMur() {
  const m = vue.module;
  for (const objet of mur.children) objet.geometry.dispose();
  mur.clear();
  const n = m.mycelium_colonie_nb_obstacles();
  const centre = new Float32Array(3);
  for (let i = 0; i < n; i++) {
    const [x0, y0, z0, x1, y1, z1] = [0, 1, 2, 3, 4, 5].map((k) => m.mycelium_colonie_obstacle(i, k));
    const [dx, dy, dz] = [x1 + 1 - x0, y1 + 1 - y0, z1 + 1 - z0];
    versScene(x0 + dx / 2, y0 + dy / 2, z0 + dz / 2, centre, 0);
    // Dans three.js, la largeur suit x, la hauteur le z du moteur et la profondeur son y.
    const boite = new THREE.BoxGeometry(dx, dz, dy);
    const plein = new THREE.Mesh(boite, matiereMur);
    const bords = new THREE.LineSegments(new THREE.EdgesGeometry(boite), matiereAretesMur);
    plein.position.set(centre[0], centre[1], centre[2]);
    bords.position.copy(plein.position);
    mur.add(plein, bords);
  }
  legendeMur.hidden = n === 0;
}

// Réserve les tampons de la colonie pour `capacite` segments, et ceux des soudures pour `places` soudures, vides.
function reserver(capacite, places) {
  colonie.geometry.dispose();
  const geometrie = new THREE.BufferGeometry();
  const positions = new THREE.BufferAttribute(new Float32Array(Math.max(1, capacite) * 6), 3);
  const couleurs = new THREE.BufferAttribute(new Float32Array(Math.max(1, capacite) * 6), 3);
  positions.setUsage(THREE.DynamicDrawUsage);
  couleurs.setUsage(THREE.DynamicDrawUsage);
  geometrie.setAttribute("position", positions);
  geometrie.setAttribute("color", couleurs);
  geometrie.setDrawRange(0, 0);
  colonie.geometry = geometrie;
  vue.ecrits = 0;
  vue.debutDernierPas = 0;
  vue.arrivees = new Float32Array(0);

  soudures.geometry.dispose();
  const lieux = new THREE.BufferGeometry();
  const points = new THREE.BufferAttribute(new Float32Array(Math.max(1, places) * 3), 3);
  points.setUsage(THREE.DynamicDrawUsage);
  lieux.setAttribute("position", points);
  lieux.setDrawRange(0, 0);
  soudures.geometry = lieux;
  vue.souduresEcrites = 0;
  vue.souduresAvantDernierPas = 0;
  legendeSoudure.hidden = places === 0;
}

// Écrit dans les tampons les segments nés depuis le dernier appel ; ils forment le dernier pas joué.
function ecrireNouveaux() {
  const m = vue.module;
  const n = m.mycelium_colonie_nb_segments();
  const positions = colonie.geometry.getAttribute("position");
  const couleurs = colonie.geometry.getAttribute("color");
  const capacite = positions.count / 2;
  const fin = Math.min(n, capacite);
  const debut = vue.ecrits;
  if (fin <= debut) return;
  const duree = Math.max(1, m.mycelium_colonie_duree() - 1);
  const p = positions.array;
  const c = couleurs.array;
  vue.arrivees = new Float32Array((fin - debut) * 3);
  const s = (i, k) => m.mycelium_colonie_segment(i, k);
  for (let i = debut; i < fin; i++) {
    const j = i * 6;
    versScene(s(i, 0), s(i, 1), s(i, 2), p, j);
    versScene(s(i, 3), s(i, 4), s(i, 5), p, j + 3);
    vue.arrivees.set(p.subarray(j + 3, j + 6), (i - debut) * 3);
    const rgb = couleur((m.mycelium_colonie_segment_temps(i) - 1) / duree);
    c.set(rgb, j);
    c.set(rgb, j + 3);
  }
  vue.croissance = 0;
  vue.allonge = -1;
  positions.addUpdateRange(debut * 6, (fin - debut) * 6);
  couleurs.addUpdateRange(debut * 6, (fin - debut) * 6);
  positions.needsUpdate = true;
  couleurs.needsUpdate = true;
  vue.debutDernierPas = debut;
  vue.ecrits = fin;
  colonie.geometry.setDrawRange(0, fin * 2);
}

// Écrit les soudures nées depuis le dernier appel ; elles sont celles du dernier pas joué.
function ecrireSoudures() {
  const m = vue.module;
  const points = soudures.geometry.getAttribute("position");
  const fin = Math.min(m.mycelium_colonie_anastomoses(), points.count);
  const debut = vue.souduresEcrites;
  vue.souduresAvantDernierPas = debut;
  if (fin <= debut) return;
  for (let i = debut; i < fin; i++) {
    versScene(
      m.mycelium_colonie_soudure(i, 0),
      m.mycelium_colonie_soudure(i, 1),
      m.mycelium_colonie_soudure(i, 2),
      points.array,
      i * 3,
    );
  }
  points.addUpdateRange(debut * 3, (fin - debut) * 3);
  points.needsUpdate = true;
  vue.souduresEcrites = fin;
}

// Allonge les segments du dernier pas à la part `f` (de 0 à 1) de leur longueur vraie ; ses soudures paraissent
// quand il est entier.
function allongerDernierPas(f) {
  soudures.geometry.setDrawRange(0, f >= 1 ? vue.souduresEcrites : vue.souduresAvantDernierPas);
  const positions = colonie.geometry.getAttribute("position");
  const p = positions.array;
  const debut = vue.debutDernierPas;
  const fin = vue.ecrits;
  if (fin <= debut || f === vue.allonge) return;
  vue.allonge = f;
  for (let i = debut; i < fin; i++) {
    const j = i * 6;
    const a = (i - debut) * 3;
    for (let k = 0; k < 3; k++) p[j + 3 + k] = p[j + k] + (vue.arrivees[a + k] - p[j + k]) * f;
  }
  positions.addUpdateRange(debut * 6, (fin - debut) * 6);
  positions.needsUpdate = true;
}

function demarrer(pasDeDepart = 0) {
  const m = vue.module;
  const numero = Number(choixScene.value);
  m.mycelium_colonie_choisir(numero);
  // Une première partie, jouée sans rien dessiner, donne le nombre final de segments et de soudures : les tampons se
  // réservent à cette taille, une fois. La colonie repart ensuite du pas 0 avec la même graine, donc à l'identique.
  m.mycelium_colonie_demarrer(vue.graine);
  m.mycelium_colonie_avancer(m.mycelium_colonie_duree());
  const capacite = m.mycelium_colonie_nb_segments();
  const places = m.mycelium_colonie_anastomoses();
  m.mycelium_colonie_demarrer(vue.graine);
  vue.dimensions = [0, 1, 2].map((k) => m.mycelium_colonie_dimension(k));
  poserAretes();
  poserMur();
  reserver(capacite, places);
  poserGoutte();
  description.textContent = DESCRIPTIONS[numero];
  vue.empreinteLue = false;
  vue.avance = 0;
  if (pasDeDepart > 0) {
    m.mycelium_colonie_avancer(Math.min(pasDeDepart, m.mycelium_colonie_duree()));
    ecrireNouveaux();
    ecrireSoudures();
    // Les pas sautés par l'adresse se montrent entiers.
    vue.croissance = 1;
  }
  vue.enLecture = m.mycelium_colonie_temps() < m.mycelium_colonie_duree();
  mettreAJourMesures();
}

// La goutte : une tache douce au centre de la case de la première nourriture du scénario, s'il y en a une.
function poserGoutte() {
  const m = vue.module;
  const point = new Float32Array(3);
  goutte.visible = m.mycelium_colonie_nb_depots() > 0;
  legendeGoutte.hidden = !goutte.visible;
  if (!goutte.visible) return;
  const [x, y, z] = [0, 1, 2].map((k) => m.mycelium_colonie_depot(0, k) + 0.5);
  versScene(x, y, z, point, 0);
  goutte.position.set(point[0], point[1], point[2]);
}

function empreinte() {
  let texte = "";
  for (let i = 0; i < 8; i++) {
    texte += (vue.module.mycelium_colonie_empreinte_mot(i) >>> 0).toString(16).padStart(8, "0");
  }
  return texte;
}

function mettreAJourMesures() {
  const m = vue.module;
  const temps = m.mycelium_colonie_temps();
  const duree = m.mycelium_colonie_duree();
  const fini = temps >= duree;
  sortiePas.textContent = `${temps} / ${duree}`;
  sortieFilaments.textContent = String(m.mycelium_colonie_nb_segments());
  sortiePointes.textContent = String(m.mycelium_colonie_pointes());
  sortieRamifications.textContent = String(m.mycelium_colonie_ramifications());
  sortieSoudures.textContent = String(m.mycelium_colonie_anastomoses());
  sortieMasse.textContent = m.mycelium_colonie_nourriture_masse().toFixed(1);
  if (fini && !vue.empreinteLue) {
    vue.empreinteLue = true;
    const e = empreinte();
    sortieEmpreinte.textContent = e;
    if (vue.graine !== GRAINE_DE_REFERENCE) {
      verdict.textContent = "Pas de référence pour cette graine : celle des tests est la graine 1.";
      verdict.className = "";
    } else if (e === REFERENCES[Number(choixScene.value)]) {
      verdict.textContent = "C'est l'empreinte de référence : la colonie du moteur natif, au bit près.";
      verdict.className = "juste";
    } else {
      verdict.textContent = "Ce n'est pas l'empreinte de référence.";
      verdict.className = "faux";
    }
  } else if (!fini) {
    sortieEmpreinte.textContent = "—";
    verdict.textContent = "Elle s'affiche à la fin de la scène.";
    verdict.className = "";
  }
  if (fini && !vue.enLecture) {
    etat.textContent = `Fin de la scène, au pas ${duree}. Glissez sur le cube pour le tourner.`;
  } else if (vue.enLecture) {
    etat.textContent = "La colonie pousse. Glissez sur le cube pour le tourner ; molette ou deux doigts pour zoomer.";
  } else {
    etat.textContent = "En pause.";
  }
  boutonLecture.textContent = vue.enLecture ? "Pause" : "Lecture";
}

// Une image : jouer les pas dus à la cadence, allonger le dernier, tourner, dessiner.
function image(maintenant) {
  const m = vue.module;
  const dt = vue.avant === null ? 0 : Math.min(0.1, (maintenant - vue.avant) / 1000);
  vue.avant = maintenant;
  if (vue.enLecture) {
    vue.avance += dt * vue.cadence;
    while (vue.avance >= 1) {
      vue.avance -= 1;
      if (m.mycelium_colonie_temps() >= m.mycelium_colonie_duree()) {
        vue.enLecture = false;
        vue.avance = 0;
        break;
      }
      allongerDernierPas(1);
      m.mycelium_colonie_avancer(1);
      ecrireNouveaux();
      ecrireSoudures();
      mettreAJourMesures();
    }
    if (!vue.enLecture) mettreAJourMesures();
  }
  // Le dernier pas ne fait que s'allonger : une pause, ou la fin, le montre entier, et il le reste.
  vue.croissance = vue.enLecture ? Math.max(vue.croissance, vue.avance) : 1;
  allongerDernierPas(vue.croissance);
  if (vue.tourneSeul && vue.pointeurs.size === 0) vue.lacet += 0.0025;
  dessiner();
  requestAnimationFrame(image);
}

function dessiner() {
  if (!rendu) return;
  const largeur = Math.max(1, Math.round(toile.clientWidth));
  const ratio = Math.min(2, window.devicePixelRatio || 1);
  if (vue.taille !== largeur || vue.ratio !== ratio) {
    vue.taille = largeur;
    vue.ratio = ratio;
    rendu.setPixelRatio(ratio);
    rendu.setSize(largeur, largeur, false);
  }
  const ct = Math.cos(vue.tangage);
  camera.position.set(
    vue.distance * ct * Math.sin(vue.lacet),
    vue.distance * Math.sin(vue.tangage),
    vue.distance * ct * Math.cos(vue.lacet),
  );
  camera.lookAt(0, 0, 0);
  scene.fog.near = vue.distance;
  scene.fog.far = vue.distance + 70;
  rendu.render(scene, camera);
}

// Tourner : un doigt ou la souris ; s'approcher : deux doigts qui s'écartent, ou la molette.
function zoomer(facteur) {
  vue.distance = Math.max(40, Math.min(170, vue.distance * facteur));
}

function ecartDesDoigts() {
  const [a, b] = Array.from(vue.pointeurs.values());
  return Math.hypot(a[0] - b[0], a[1] - b[1]);
}

toile.addEventListener("pointerdown", (evenement) => {
  vue.pointeurs.set(evenement.pointerId, [evenement.clientX, evenement.clientY]);
  if (vue.pointeurs.size === 2) vue.ecart = ecartDesDoigts();
  vue.tourneSeul = false;
  toile.setPointerCapture(evenement.pointerId);
  toile.classList.add("tenu");
});
toile.addEventListener("pointermove", (evenement) => {
  const avant = vue.pointeurs.get(evenement.pointerId);
  if (!avant) return;
  vue.pointeurs.set(evenement.pointerId, [evenement.clientX, evenement.clientY]);
  if (vue.pointeurs.size === 1) {
    vue.lacet -= (evenement.clientX - avant[0]) * 0.008;
    vue.tangage = Math.max(-1.45, Math.min(1.45, vue.tangage + (evenement.clientY - avant[1]) * 0.008));
  } else if (vue.pointeurs.size === 2) {
    const ecart = ecartDesDoigts();
    if (vue.ecart > 0 && ecart > 0) zoomer(vue.ecart / ecart);
    vue.ecart = ecart;
  }
});
for (const fin of ["pointerup", "pointercancel"]) {
  toile.addEventListener(fin, (evenement) => {
    vue.pointeurs.delete(evenement.pointerId);
    vue.ecart = vue.pointeurs.size === 2 ? ecartDesDoigts() : 0;
    if (vue.pointeurs.size === 0) toile.classList.remove("tenu");
  });
}
toile.addEventListener(
  "wheel",
  (evenement) => {
    evenement.preventDefault();
    zoomer(Math.exp(evenement.deltaY * 0.001));
  },
  { passive: false },
);

boutonLecture.addEventListener("click", () => {
  const m = vue.module;
  if (!m) return;
  if (m.mycelium_colonie_temps() >= m.mycelium_colonie_duree() && !vue.enLecture) {
    demarrer();
    return;
  }
  vue.enLecture = !vue.enLecture;
  vue.avance = 0;
  mettreAJourMesures();
});
boutonRecommencer.addEventListener("click", () => vue.module && demarrer());
choixScene.addEventListener("change", () => vue.module && demarrer());
curseurVitesse.addEventListener("input", () => {
  vue.cadence = Number(curseurVitesse.value);
  vitesseLue.textContent = vue.cadence === 1 ? "1 pas par seconde" : `${vue.cadence} pas par seconde`;
});

// Lit un texte en graine : l'entier de 0 à GRAINE_MAX qu'il écrit en chiffres, ou null.
function lireGraine(texte) {
  if (typeof texte !== "string" || !/^[0-9]+$/.test(texte.trim())) return null;
  const valeur = Number(texte.trim());
  return Number.isSafeInteger(valeur) && valeur <= GRAINE_MAX ? valeur : null;
}
champGraine.addEventListener("change", () => {
  const graine = lireGraine(champGraine.value);
  champGraine.setAttribute("aria-invalid", String(graine === null));
  if (graine === null || !vue.module) return;
  vue.graine = graine;
  demarrer();
});

// L'adresse peut choisir la scène (de 0 à 4), le pas de départ et la graine : `colonie.html?scene=3&pas=100&graine=1`
// ouvre la fin de la scène du mur.
const adresse = new URLSearchParams(window.location.search);
const sceneDemandee = Number(adresse.get("scene"));
if (
  adresse.get("scene") !== null &&
  Number.isInteger(sceneDemandee) &&
  sceneDemandee >= 0 &&
  sceneDemandee < DESCRIPTIONS.length
) {
  choixScene.value = String(sceneDemandee);
}
const pasDemande = Math.max(0, Math.floor(Number(adresse.get("pas")) || 0));
const graineDemandee = lireGraine(adresse.get("graine") ?? "");
if (graineDemandee !== null) {
  vue.graine = graineDemandee;
  champGraine.value = String(graineDemandee);
}

async function charger() {
  if (!rendu) {
    etat.textContent = "Ce navigateur n'a pas pu ouvrir WebGL : la colonie ne peut pas se dessiner ici.";
    return;
  }
  rendu.setClearColor(0x040607, 1);
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
