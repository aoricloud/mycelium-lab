// Lit la colonie de l'étape 20 par le module WebAssembly de la vitrine.
//
// Usage : node colonie-wasm.mjs <fichier.wasm>
//
// Le script charge le module (sans aucune importation), lit d'abord ce qu'il rend avant tout démarrage, vérifie que
// `mycelium_colonie_choisir(9)` rend 0 sans rien changer, puis fait tourner les deux scènes, la 0 puis la 1, à la
// graine 1 : pour chacune, il choisit (`mycelium_colonie_choisir` doit rendre 1), démarre
// (`mycelium_colonie_demarrer`), puis avance par morceaux : 1, 0, 9 pas, puis le reste de la durée lue
// (`mycelium_colonie_duree`). Il écrit sur la sortie standard, une par ligne, `clé=valeur`, dans cet ordre :
//
//   avant                  : ce que rendent les fonctions avant tout démarrage, séparés par des virgules : `temps`,
//                            `duree`, `dimension(0)`, `dimension(1)`, `dimension(2)`, `dimension(3)`,
//                            `nb_segments`, `segment(0, 0)`, `segment(0, 5)`, `segment_temps(0)`, `pointes`,
//                            `ramifications`, `nourriture(0, 0, 0)`, `nourriture_maximum`, `nourriture_masse`,
//                            `nb_depots`, `depot(0, 0)`, `depot(0, 3)`, `empreinte_mot(0)`, `empreinte_mot(8)`
//                            (`0,0,0,0,0,0,0,NaN,NaN,0,0,0,NaN,NaN,NaN,0,NaN,NaN,0,0` attendu) ;
//   apres_choix_refuse     : les mêmes vingt valeurs, relues après `mycelium_colonie_choisir(9)`, qui rend 0 et ne
//                            change rien ;
//   scenario_<n>_depart    : pour chaque scène `n` (0 puis 1), l'état juste après le démarrage, avant tout pas ;
//   scenario_<n>_<i>       : l'état après le morceau `i` (de 0) avancé ; les morceaux sont `1,0,9,duree-10` ;
//   graine_2_scene_0       : l'empreinte de la scène 0 à la graine 2 après sa durée entière ; elle diffère de celle
//                            de la graine 1 (le script le vérifie) ;
//   chaque ligne d'état    : `temps,duree,largeur,hauteur,profondeur,nb_segments,pointes,ramifications,bits_masse,
//                            bits_maximum,hache_segments,hache_temps,depots,empreinte`, où les deux `bits` sont les 16
//                            chiffres hexadécimaux minuscules du motif de bits du f64, `hache_segments` le SHA-256
//                            (64 chiffres hexadécimaux minuscules) des f64 petit-boutistes de `segment(i, 0)` à
//                            `segment(i, 5)`, segment après segment dans l'ordre, `hache_temps` le SHA-256 des `u32`
//                            petit-boutistes de `segment_temps(i)`, dans le même ordre, `depots` le nombre de dépôts
//                            suivi de chacun par `:x:y:z:bits_quantite`, et `empreinte` les huit mots de
//                            `mycelium_colonie_empreinte_mot`, chacun sur 8 chiffres, mis bout à bout.
//
// Hors du volume, `mycelium_colonie_nourriture` doit rendre `NaN`, et au-delà du dernier segment
// `mycelium_colonie_segment` aussi, à chaque état ; sinon le script écrit un message sur la sortie d'erreur et sort
// avec le code 1. Si le fichier ou les arguments sont faux, ou si une exportation manque, il fait de même. Aucune
// dépendance : les modules intégrés de Node seulement.

import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";

const EXPORTATIONS = [
  "mycelium_colonie_choisir",
  "mycelium_colonie_demarrer",
  "mycelium_colonie_avancer",
  "mycelium_colonie_temps",
  "mycelium_colonie_duree",
  "mycelium_colonie_dimension",
  "mycelium_colonie_nb_segments",
  "mycelium_colonie_segment",
  "mycelium_colonie_segment_temps",
  "mycelium_colonie_pointes",
  "mycelium_colonie_ramifications",
  "mycelium_colonie_nourriture",
  "mycelium_colonie_nourriture_maximum",
  "mycelium_colonie_nourriture_masse",
  "mycelium_colonie_nb_depots",
  "mycelium_colonie_depot",
  "mycelium_colonie_empreinte_mot",
];
const MAX_U32 = 4294967295;
const SCENES = [0, 1];
const GRAINE = 1;

/** Le motif de bits d'un f64, en 16 chiffres hexadécimaux minuscules. */
const vue = new DataView(new ArrayBuffer(8));
function bitsHex(valeur) {
  vue.setFloat64(0, valeur, true);
  return vue.getBigUint64(0, true).toString(16).padStart(16, "0");
}

/** Le SHA-256 en hexadécimal d'une suite de f64, petit-boutistes, mis bout à bout. */
function hacher(valeurs) {
  const octets = Buffer.alloc(8 * valeurs.length);
  valeurs.forEach((valeur, i) => octets.writeDoubleLE(valeur, 8 * i));
  return createHash("sha256").update(octets).digest("hex");
}

/** Le SHA-256 en hexadécimal d'une suite d'entiers de 32 bits, petit-boutistes, mis bout à bout. */
function hacherU32(valeurs) {
  const octets = Buffer.alloc(4 * valeurs.length);
  valeurs.forEach((valeur, i) => octets.writeUInt32LE(valeur, 4 * i));
  return createHash("sha256").update(octets).digest("hex");
}

async function principal() {
  const args = process.argv.slice(2);
  if (args.length !== 1) {
    throw new Error("usage : node colonie-wasm.mjs <fichier.wasm>");
  }

  const octets = readFileSync(args[0]);
  const { instance } = await WebAssembly.instantiate(octets, {});
  const m = instance.exports;
  for (const nom of EXPORTATIONS) {
    if (typeof m[nom] !== "function") {
      throw new Error(`le module n'exporte pas la fonction ${nom}`);
    }
  }

  const lignes = [];

  // Les lectures d'avant tout démarrage : les entiers rendent 0, les flottants `NaN`.
  const etatAvant = () =>
    [
      m.mycelium_colonie_temps() >>> 0,
      m.mycelium_colonie_duree() >>> 0,
      m.mycelium_colonie_dimension(0) >>> 0,
      m.mycelium_colonie_dimension(1) >>> 0,
      m.mycelium_colonie_dimension(2) >>> 0,
      m.mycelium_colonie_dimension(3) >>> 0,
      m.mycelium_colonie_nb_segments() >>> 0,
      m.mycelium_colonie_segment(0, 0),
      m.mycelium_colonie_segment(0, 5),
      m.mycelium_colonie_segment_temps(0) >>> 0,
      m.mycelium_colonie_pointes() >>> 0,
      m.mycelium_colonie_ramifications() >>> 0,
      m.mycelium_colonie_nourriture(0, 0, 0),
      m.mycelium_colonie_nourriture_maximum(),
      m.mycelium_colonie_nourriture_masse(),
      m.mycelium_colonie_nb_depots() >>> 0,
      m.mycelium_colonie_depot(0, 0),
      m.mycelium_colonie_depot(0, 3),
      m.mycelium_colonie_empreinte_mot(0) >>> 0,
      m.mycelium_colonie_empreinte_mot(8) >>> 0,
    ].join(",");

  const avant = etatAvant();
  lignes.push(`avant=${avant}`);

  if (m.mycelium_colonie_choisir(9) >>> 0 !== 0) {
    throw new Error("le module accepte le numéro de scène 9");
  }
  const apresChoixRefuse = etatAvant();
  if (apresChoixRefuse !== avant) {
    throw new Error("choisir(9) a changé l'état alors qu'il devait rendre 0 sans rien changer");
  }
  lignes.push(`apres_choix_refuse=${apresChoixRefuse}`);

  /** Les huit mots de l'empreinte, lus en 64 chiffres hexadécimaux minuscules. */
  const empreinte = () => {
    let texte = "";
    for (let i = 0; i < 8; i++) {
      texte += (m.mycelium_colonie_empreinte_mot(i) >>> 0).toString(16).padStart(8, "0");
    }
    return texte;
  };

  /** Le nombre de dépôts, puis `:x:y:z:bits_quantite` pour chacun. */
  const depots = () => {
    const n = m.mycelium_colonie_nb_depots() >>> 0;
    let texte = String(n);
    for (let i = 0; i < n; i++) {
      texte +=
        `:${m.mycelium_colonie_depot(i, 0) >>> 0}` +
        `:${m.mycelium_colonie_depot(i, 1) >>> 0}` +
        `:${m.mycelium_colonie_depot(i, 2) >>> 0}` +
        `:${bitsHex(m.mycelium_colonie_depot(i, 3))}`;
    }
    return texte;
  };

  /** Vérifie les bords d'un état : hors du volume `nourriture` rend `NaN`, au-delà du dernier segment `segment`
   * aussi, et les lectures hors de portée rendent 0. */
  const verifierBords = (nom, largeur, hauteur, profondeur, nbSegments) => {
    const dehors = [
      [largeur, 0, 0],
      [0, hauteur, 0],
      [0, 0, profondeur],
      [largeur, hauteur, profondeur],
      [MAX_U32, MAX_U32, MAX_U32],
    ];
    for (const [x, y, z] of dehors) {
      if (!Number.isNaN(m.mycelium_colonie_nourriture(x, y, z))) {
        throw new Error(`${nom} : nourriture(${x}, ${y}, ${z}) n'est pas NaN hors du volume`);
      }
    }
    for (const [i, k] of [
      [nbSegments, 0],
      [nbSegments, 5],
      [nbSegments, MAX_U32],
    ]) {
      if (!Number.isNaN(m.mycelium_colonie_segment(i, k))) {
        throw new Error(`${nom} : segment(${i}, ${k}) n'est pas NaN au-delà du dernier segment`);
      }
    }
    if (nbSegments > 0 && !Number.isNaN(m.mycelium_colonie_segment(0, 6))) {
      throw new Error(`${nom} : segment(0, 6) n'est pas NaN pour un k hors de portée`);
    }
    if ((m.mycelium_colonie_segment_temps(nbSegments) >>> 0) !== 0) {
      throw new Error(`${nom} : segment_temps(${nbSegments}) n'est pas 0 au-delà du dernier segment`);
    }
    if ((m.mycelium_colonie_dimension(3) >>> 0) !== 0 || (m.mycelium_colonie_dimension(MAX_U32) >>> 0) !== 0) {
      throw new Error(`${nom} : dimension rend autre chose que 0 pour un k hors de portée`);
    }
    if (!Number.isNaN(m.mycelium_colonie_depot(MAX_U32, 0)) || !Number.isNaN(m.mycelium_colonie_depot(0, 4))) {
      throw new Error(`${nom} : depot ne rend pas NaN pour un dépôt ou un champ hors de portée`);
    }
    if ((m.mycelium_colonie_empreinte_mot(8) >>> 0) !== 0 || (m.mycelium_colonie_empreinte_mot(MAX_U32) >>> 0) !== 0) {
      throw new Error(`${nom} : empreinte_mot rend autre chose que 0 au-delà du huitième mot`);
    }
  };

  /** Écrit la ligne d'état `nom`, puis vérifie ses bords. */
  const lire = (nom) => {
    const temps = m.mycelium_colonie_temps() >>> 0;
    const duree = m.mycelium_colonie_duree() >>> 0;
    const largeur = m.mycelium_colonie_dimension(0) >>> 0;
    const hauteur = m.mycelium_colonie_dimension(1) >>> 0;
    const profondeur = m.mycelium_colonie_dimension(2) >>> 0;
    const nbSegments = m.mycelium_colonie_nb_segments() >>> 0;
    const pointes = m.mycelium_colonie_pointes() >>> 0;
    const ramifications = m.mycelium_colonie_ramifications() >>> 0;
    const masse = m.mycelium_colonie_nourriture_masse();
    const maximum = m.mycelium_colonie_nourriture_maximum();
    const coordonnees = [];
    const naissances = [];
    for (let i = 0; i < nbSegments; i++) {
      for (let k = 0; k < 6; k++) {
        coordonnees.push(m.mycelium_colonie_segment(i, k));
      }
      naissances.push(m.mycelium_colonie_segment_temps(i) >>> 0);
    }
    lignes.push(
      `${nom}=${temps},${duree},${largeur},${hauteur},${profondeur},${nbSegments},${pointes},${ramifications},` +
        `${bitsHex(masse)},${bitsHex(maximum)},${hacher(coordonnees)},${hacherU32(naissances)},${depots()},` +
        `${empreinte()}`,
    );
    verifierBords(nom, largeur, hauteur, profondeur, nbSegments);
  };

  let empreinteScene0Graine1 = null;
  for (const scene of SCENES) {
    if (m.mycelium_colonie_choisir(scene) >>> 0 !== 1) {
      throw new Error(`le module refuse la scène ${scene}`);
    }
    m.mycelium_colonie_demarrer(GRAINE);
    const prefixe = `scenario_${scene}`;
    lire(`${prefixe}_depart`);

    const duree = m.mycelium_colonie_duree() >>> 0;
    if (duree < 10) {
      throw new Error(`la scène ${scene} dure ${duree} pas : trop peu pour les morceaux demandés`);
    }
    [1, 0, 9, duree - 10].forEach((pas, i) => {
      m.mycelium_colonie_avancer(pas);
      lire(`${prefixe}_${i}`);
    });
    if (scene === 0) {
      empreinteScene0Graine1 = empreinte();
    }
  }

  // La graine compte : la scène 0 à la graine 2 ne redonne pas l'empreinte de la graine 1.
  if (m.mycelium_colonie_choisir(0) >>> 0 !== 1) {
    throw new Error("le module refuse la scène 0 au second démarrage");
  }
  m.mycelium_colonie_demarrer(2);
  m.mycelium_colonie_avancer(m.mycelium_colonie_duree() >>> 0);
  const empreinteGraine2 = empreinte();
  if (empreinteGraine2 === empreinteScene0Graine1) {
    throw new Error("la scène 0 rend la même empreinte aux graines 1 et 2");
  }
  lignes.push(`graine_2_scene_0=${empreinteGraine2}`);

  process.stdout.write(lignes.join("\n") + "\n");
}

principal().catch((erreur) => {
  process.stderr.write(`colonie-wasm : ${erreur.message}\n`);
  process.exitCode = 1;
});
