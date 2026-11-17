// Lit le cube de l'étape 18 par le module WebAssembly de la vitrine.
//
// Usage : node cube-wasm.mjs <fichier.wasm>
//
// Le script charge le module (sans aucune importation), lit d'abord ce qu'il rend avant tout démarrage, vérifie que
// `mycelium_cube_choisir(2)` rend 0 sans rien changer, puis fait tourner les deux scènes, la 0 puis la 1 : pour
// chacune, il choisit (`mycelium_cube_choisir` doit rendre 1), démarre (`mycelium_cube_demarrer`), puis avance par
// morceaux : 1, 0, 49 pas, puis le reste de la durée lue (`mycelium_cube_duree`). Il écrit sur la sortie standard,
// une par ligne, `clé=valeur`, dans cet ordre :
//
//   avant                  : ce que rendent les fonctions avant tout démarrage, séparés par des virgules : `temps`,
//                            `duree`, `dimension(0)`, `dimension(1)`, `dimension(2)`, `dimension(3)`,
//                            `valeur(0, 0, 0)`, `bloquee(0, 0, 0)`, `maximum`, `masse`, `empreinte_mot(0)`,
//                            `empreinte_mot(8)` (`0,0,0,0,0,0,NaN,0,NaN,NaN,0,0` attendu) ;
//   apres_choix_refuse     : les mêmes douze valeurs, relues après `mycelium_cube_choisir(2)`, qui rend 0 et ne
//                            change rien ;
//   scenario_<n>_depart    : pour chaque scène `n` (0 puis 1), l'état juste après le démarrage, avant tout pas ;
//   scenario_<n>_<i>       : l'état après le morceau `i` (de 0) avancé ; les morceaux sont `1,0,49,duree-50` ;
//   chaque ligne d'état    : `temps,duree,largeur,hauteur,profondeur,bits_masse,bits_maximum,hache_valeurs,
//                            bloquees,empreinte`, où les deux `bits` sont les 16 chiffres hexadécimaux minuscules du
//                            motif de bits du f64, `hache_valeurs` le SHA-256 (64 chiffres hexadécimaux minuscules)
//                            des f64 petit-boutistes de `mycelium_cube_valeur(x, y, z)` parcourues `z` lent, puis
//                            `y`, puis `x` rapide, `bloquees` le nombre de cases dont `mycelium_cube_bloquee` rend 1,
//                            et `empreinte` les huit mots de `mycelium_cube_empreinte_mot`, chacun sur 8 chiffres,
//                            mis bout à bout.
//
// Hors du volume, `mycelium_cube_valeur` doit rendre `NaN` et `mycelium_cube_bloquee` 0, à chaque état ; sinon le
// script écrit un message sur la sortie d'erreur et sort avec le code 1. Si le fichier ou les arguments sont faux,
// ou si une exportation manque, il fait de même. Aucune dépendance : les modules intégrés de Node seulement.

import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";

const EXPORTATIONS = [
  "mycelium_cube_choisir",
  "mycelium_cube_demarrer",
  "mycelium_cube_avancer",
  "mycelium_cube_temps",
  "mycelium_cube_duree",
  "mycelium_cube_dimension",
  "mycelium_cube_valeur",
  "mycelium_cube_bloquee",
  "mycelium_cube_maximum",
  "mycelium_cube_masse",
  "mycelium_cube_empreinte_mot",
];
const MAX_U32 = 4294967295;
const SCENES = [0, 1];

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

async function principal() {
  const args = process.argv.slice(2);
  if (args.length !== 1) {
    throw new Error("usage : node cube-wasm.mjs <fichier.wasm>");
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
      m.mycelium_cube_temps() >>> 0,
      m.mycelium_cube_duree() >>> 0,
      m.mycelium_cube_dimension(0) >>> 0,
      m.mycelium_cube_dimension(1) >>> 0,
      m.mycelium_cube_dimension(2) >>> 0,
      m.mycelium_cube_dimension(3) >>> 0,
      m.mycelium_cube_valeur(0, 0, 0),
      m.mycelium_cube_bloquee(0, 0, 0) >>> 0,
      m.mycelium_cube_maximum(),
      m.mycelium_cube_masse(),
      m.mycelium_cube_empreinte_mot(0) >>> 0,
      m.mycelium_cube_empreinte_mot(8) >>> 0,
    ].join(",");

  const avant = etatAvant();
  lignes.push(`avant=${avant}`);

  if (m.mycelium_cube_choisir(2) >>> 0 !== 0) {
    throw new Error("le module accepte le numéro de scène 2");
  }
  const apresChoixRefuse = etatAvant();
  if (apresChoixRefuse !== avant) {
    throw new Error("choisir(2) a changé l'état alors qu'il devait rendre 0 sans rien changer");
  }
  lignes.push(`apres_choix_refuse=${apresChoixRefuse}`);

  /** Les huit mots de l'empreinte, lus en 64 chiffres hexadécimaux minuscules. */
  const empreinte = () => {
    let texte = "";
    for (let i = 0; i < 8; i++) {
      texte += (m.mycelium_cube_empreinte_mot(i) >>> 0).toString(16).padStart(8, "0");
    }
    return texte;
  };

  /** Vérifie les bords d'un état : hors du volume, `valeur` rend `NaN` et `bloquee` 0 ; `dimension` et
   * `empreinte_mot` rendent 0 au-delà de leur portée. */
  const verifierBords = (nom, largeur, hauteur, profondeur) => {
    const dehors = [
      [largeur, 0, 0],
      [0, hauteur, 0],
      [0, 0, profondeur],
      [largeur, hauteur, profondeur],
      [MAX_U32, MAX_U32, MAX_U32],
    ];
    for (const [x, y, z] of dehors) {
      if (!Number.isNaN(m.mycelium_cube_valeur(x, y, z))) {
        throw new Error(`${nom} : valeur(${x}, ${y}, ${z}) n'est pas NaN hors du volume`);
      }
      if ((m.mycelium_cube_bloquee(x, y, z) >>> 0) !== 0) {
        throw new Error(`${nom} : bloquee(${x}, ${y}, ${z}) n'est pas 0 hors du volume`);
      }
    }
    if ((m.mycelium_cube_dimension(3) >>> 0) !== 0 || (m.mycelium_cube_dimension(MAX_U32) >>> 0) !== 0) {
      throw new Error(`${nom} : dimension rend autre chose que 0 pour un k hors de portée`);
    }
    if ((m.mycelium_cube_empreinte_mot(8) >>> 0) !== 0 || (m.mycelium_cube_empreinte_mot(MAX_U32) >>> 0) !== 0) {
      throw new Error(`${nom} : empreinte_mot rend autre chose que 0 au-delà du huitième mot`);
    }
  };

  /** Écrit la ligne d'état `nom`, puis vérifie ses bords. */
  const lire = (nom) => {
    const temps = m.mycelium_cube_temps() >>> 0;
    const duree = m.mycelium_cube_duree() >>> 0;
    const largeur = m.mycelium_cube_dimension(0) >>> 0;
    const hauteur = m.mycelium_cube_dimension(1) >>> 0;
    const profondeur = m.mycelium_cube_dimension(2) >>> 0;
    const masse = m.mycelium_cube_masse();
    const maximum = m.mycelium_cube_maximum();
    const valeurs = [];
    let bloquees = 0;
    for (let z = 0; z < profondeur; z++) {
      for (let y = 0; y < hauteur; y++) {
        for (let x = 0; x < largeur; x++) {
          valeurs.push(m.mycelium_cube_valeur(x, y, z));
          bloquees += m.mycelium_cube_bloquee(x, y, z) >>> 0;
        }
      }
    }
    lignes.push(
      `${nom}=${temps},${duree},${largeur},${hauteur},${profondeur},${bitsHex(masse)},${bitsHex(maximum)},` +
        `${hacher(valeurs)},${bloquees},${empreinte()}`,
    );
    verifierBords(nom, largeur, hauteur, profondeur);
  };

  for (const scene of SCENES) {
    if (m.mycelium_cube_choisir(scene) >>> 0 !== 1) {
      throw new Error(`le module refuse la scène ${scene}`);
    }
    m.mycelium_cube_demarrer();
    const prefixe = `scenario_${scene}`;
    lire(`${prefixe}_depart`);

    const duree = m.mycelium_cube_duree() >>> 0;
    if (duree < 50) {
      throw new Error(`la scène ${scene} dure ${duree} pas : trop peu pour les morceaux demandés`);
    }
    [1, 0, 49, duree - 50].forEach((pas, i) => {
      m.mycelium_cube_avancer(pas);
      lire(`${prefixe}_${i}`);
    });
  }

  process.stdout.write(lignes.join("\n") + "\n");
}

principal().catch((erreur) => {
  process.stderr.write(`cube-wasm : ${erreur.message}\n`);
  process.exitCode = 1;
});
