// Lit les scènes 5, 6, 7 et 8 de la colonie en volume par le module WebAssembly de la vitrine (étape 28).
//
// Usage : node scenes-wasm.mjs <fichier.wasm>
//
// Le script charge le module (sans aucune importation), lit la part de la cavité atteinte avant tout démarrage (NaN),
// vérifie que `mycelium_colonie_choisir(9)` et `mycelium_colonie_choisir(4294967295)` rendent 0 sans rien changer,
// puis fait tourner les scènes 5 (la boule), 6 (le tore), 7 (la coquille) et 8 (le labyrinthe), à la graine 1 : pour
// chacune, il choisit (`mycelium_colonie_choisir` doit rendre 1), démarre (`mycelium_colonie_demarrer`), puis avance
// par morceaux : 1, 0, 9 pas, puis le reste de la durée lue (`mycelium_colonie_duree`). Il écrit sur la sortie
// standard, une par ligne, `clé=valeur`, dans cet ordre :
//
//   avant                  : la part de la cavité atteinte avant tout démarrage, par les 16 chiffres hexadécimaux
//                            minuscules du motif de bits de son f64 (`7ff8000000000000`, NaN, attendu) ;
//   scenario_<n>_depart    : pour chaque scène `n` (5, 6, 7 puis 8), l'état juste après le démarrage, avant tout pas ;
//   scenario_<n>_<i>       : l'état après le morceau `i` (de 0) avancé ; les morceaux sont `1,0,9,duree-10` ;
//   chaque ligne d'état    : `temps,duree,largeur,hauteur,profondeur,nb_segments,pointes,ramifications,nb_obstacles,
//                            hache_obstacles,bits_part,empreinte`, où `hache_obstacles` est le SHA-256 (64 chiffres
//                            hexadécimaux minuscules) des coordonnées des obstacles, six entiers u32 petit-boutistes par
//                            obstacle, dans l'ordre du scénario, `bits_part` les 16 chiffres hexadécimaux minuscules du
//                            motif de bits du f64 de la part remplie, et `empreinte` les huit mots de
//                            `mycelium_colonie_empreinte_mot`, chacun sur 8 chiffres hexadécimaux minuscules, mis bout
//                            à bout.
//
// Si le fichier ou les arguments sont faux, si une exportation manque, si un numéro refusé est accepté ou change
// l'état, le script écrit un message sur la sortie d'erreur et sort avec le code 1. Aucune dépendance : les modules
// intégrés de Node seulement.

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
  "mycelium_colonie_pointes",
  "mycelium_colonie_ramifications",
  "mycelium_colonie_nb_obstacles",
  "mycelium_colonie_obstacle",
  "mycelium_colonie_part_remplie",
  "mycelium_colonie_empreinte_mot",
];
const MAX_U32 = 4294967295;
const SCENES = [5, 6, 7, 8];
const GRAINE = 1;

/** Le motif de bits d'un f64, en 16 chiffres hexadécimaux minuscules. */
const vue = new DataView(new ArrayBuffer(8));
function bitsHex(valeur) {
  vue.setFloat64(0, valeur, true);
  return vue.getBigUint64(0, true).toString(16).padStart(16, "0");
}

/** Le SHA-256 en hexadécimal d'une suite d'entiers de 32 bits, petit-boutistes, mis bout à bout. */
function hacherU32(valeurs) {
  const octets = Buffer.alloc(4 * valeurs.length);
  valeurs.forEach((valeur, i) => octets.writeUInt32LE(valeur >>> 0, 4 * i));
  return createHash("sha256").update(octets).digest("hex");
}

async function principal() {
  const args = process.argv.slice(2);
  if (args.length !== 1) {
    throw new Error("usage : node scenes-wasm.mjs <fichier.wasm>");
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

  // La part de la cavité atteinte avant tout démarrage : `NaN`.
  const partAvant = bitsHex(m.mycelium_colonie_part_remplie());
  lignes.push(`avant=${partAvant}`);

  // Les deux numéros refusés rendent 0 et ne changent rien.
  for (const numero of [9, MAX_U32]) {
    if (m.mycelium_colonie_choisir(numero) >>> 0 !== 0) {
      throw new Error(`le module accepte le numéro de scène ${numero}`);
    }
    if (bitsHex(m.mycelium_colonie_part_remplie()) !== partAvant) {
      throw new Error(`choisir(${numero}) a changé l'état alors qu'il devait rendre 0 sans rien changer`);
    }
  }

  /** Les huit mots de l'empreinte, lus en 64 chiffres hexadécimaux minuscules. */
  const empreinte = () => {
    let texte = "";
    for (let i = 0; i < 8; i++) {
      texte += (m.mycelium_colonie_empreinte_mot(i) >>> 0).toString(16).padStart(8, "0");
    }
    return texte;
  };

  /** Le SHA-256 des coordonnées des `n` obstacles, six u32 petit-boutistes chacun, dans l'ordre du scénario. */
  const hacherObstacles = (n) => {
    const valeurs = [];
    for (let i = 0; i < n; i++) {
      for (let k = 0; k < 6; k++) {
        valeurs.push(m.mycelium_colonie_obstacle(i, k) >>> 0);
      }
    }
    return hacherU32(valeurs);
  };

  /** Écrit la ligne d'état `nom`. */
  const lire = (nom) => {
    const temps = m.mycelium_colonie_temps() >>> 0;
    const duree = m.mycelium_colonie_duree() >>> 0;
    const largeur = m.mycelium_colonie_dimension(0) >>> 0;
    const hauteur = m.mycelium_colonie_dimension(1) >>> 0;
    const profondeur = m.mycelium_colonie_dimension(2) >>> 0;
    const nbSegments = m.mycelium_colonie_nb_segments() >>> 0;
    const pointes = m.mycelium_colonie_pointes() >>> 0;
    const ramifications = m.mycelium_colonie_ramifications() >>> 0;
    const nbObstacles = m.mycelium_colonie_nb_obstacles() >>> 0;
    const part = bitsHex(m.mycelium_colonie_part_remplie());
    lignes.push(
      `${nom}=${temps},${duree},${largeur},${hauteur},${profondeur},${nbSegments},${pointes},${ramifications},` +
        `${nbObstacles},${hacherObstacles(nbObstacles)},${part},${empreinte()}`,
    );
  };

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
  }

  process.stdout.write(lignes.join("\n") + "\n");
}

principal().catch((erreur) => {
  process.stderr.write(`scenes-wasm : ${erreur.message}\n`);
  process.exitCode = 1;
});
