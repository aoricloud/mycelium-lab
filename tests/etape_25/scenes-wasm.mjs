// Lit les scènes 2, 3 et 4 de la colonie en volume par le module WebAssembly de la vitrine.
//
// Usage : node scenes-wasm.mjs <fichier.wasm>
//
// Le script charge le module (sans aucune importation), lit les quatre fonctions nouvelles avant tout démarrage
// (`nb_obstacles` 0, `obstacle(0, 0)` NaN, `anastomoses` 0, `soudure(0, 0)` NaN), vérifie que
// `mycelium_colonie_choisir(5)` et `mycelium_colonie_choisir(4294967295)` rendent 0 sans rien changer, puis fait
// tourner les scènes 2, 3 et 4, à la graine 1 : pour chacune, il choisit (`mycelium_colonie_choisir` doit rendre 1),
// démarre (`mycelium_colonie_demarrer`), puis avance par morceaux : 1, 0, 9 pas, puis le reste de la durée lue
// (`mycelium_colonie_duree`). Il écrit sur la sortie standard, une par ligne, `clé=valeur`, dans cet ordre :
//
//   avant                  : les quatre lectures d'avant tout démarrage, séparées par des virgules : `nb_obstacles`,
//                            `obstacle(0, 0)`, `anastomoses`, `soudure(0, 0)` (`0,NaN,0,NaN` attendu) ;
//   scenario_<n>_depart    : pour chaque scène `n` (2, 3 puis 4), l'état juste après le démarrage, avant tout pas ;
//   scenario_<n>_<i>       : l'état après le morceau `i` (de 0) avancé ; les morceaux sont `1,0,9,duree-10` ;
//   chaque ligne d'état    : `temps,duree,nb_segments,pointes,ramifications,anastomoses,hache_segments,obstacles,
//                            hache_soudures,empreinte`, où `hache_segments` est le SHA-256 (64 chiffres hexadécimaux
//                            minuscules) des f64 petit-boutistes des six coordonnées de chaque segment, dans l'ordre,
//                            `obstacles` le nombre d'obstacles suivi de `:x0:y0:z0:x1:y1:z1` pour chacun, en entiers,
//                            `hache_soudures` le SHA-256 des f64 petit-boutistes des trois coordonnées de chaque
//                            soudure, dans l'ordre, et `empreinte` les huit mots de `mycelium_colonie_empreinte_mot`,
//                            chacun sur 8 chiffres hexadécimaux minuscules, mis bout à bout.
//
// À chaque état, `obstacle(nb_obstacles, 0)` et `soudure(anastomoses, 0)` doivent rendre `NaN`, et s'il y en a,
// `obstacle(0, 6)` et `soudure(0, 3)` aussi ; sinon le script écrit un message sur la sortie d'erreur et sort avec
// le code 1. Si le fichier ou les arguments sont faux, ou si une exportation manque, il fait de même. Aucune
// dépendance : les modules intégrés de Node seulement.

import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";

const EXPORTATIONS = [
  "mycelium_colonie_choisir",
  "mycelium_colonie_demarrer",
  "mycelium_colonie_avancer",
  "mycelium_colonie_temps",
  "mycelium_colonie_duree",
  "mycelium_colonie_nb_segments",
  "mycelium_colonie_segment",
  "mycelium_colonie_pointes",
  "mycelium_colonie_ramifications",
  "mycelium_colonie_anastomoses",
  "mycelium_colonie_nb_obstacles",
  "mycelium_colonie_obstacle",
  "mycelium_colonie_soudure",
  "mycelium_colonie_empreinte_mot",
];
const MAX_U32 = 4294967295;
const SCENES = [2, 3, 4];
const GRAINE = 1;

/** Le SHA-256 en hexadécimal d'une suite de f64, petit-boutistes, mis bout à bout. */
function hacher(valeurs) {
  const octets = Buffer.alloc(8 * valeurs.length);
  valeurs.forEach((valeur, i) => octets.writeDoubleLE(valeur, 8 * i));
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

  // Les quatre lectures d'avant tout démarrage : les entiers rendent 0, les flottants `NaN`.
  const etatAvant = () =>
    [
      m.mycelium_colonie_nb_obstacles() >>> 0,
      m.mycelium_colonie_obstacle(0, 0),
      m.mycelium_colonie_anastomoses() >>> 0,
      m.mycelium_colonie_soudure(0, 0),
    ].join(",");

  const avant = etatAvant();
  lignes.push(`avant=${avant}`);

  // Les deux numéros refusés rendent 0 et ne changent rien.
  for (const numero of [5, MAX_U32]) {
    if (m.mycelium_colonie_choisir(numero) >>> 0 !== 0) {
      throw new Error(`le module accepte le numéro de scène ${numero}`);
    }
    if (etatAvant() !== avant) {
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

  /** Le nombre d'obstacles, puis `:x0:y0:z0:x1:y1:z1` pour chacun, en entiers, dans l'ordre du scénario. */
  const obstacles = () => {
    const n = m.mycelium_colonie_nb_obstacles() >>> 0;
    let texte = String(n);
    for (let i = 0; i < n; i++) {
      for (let k = 0; k < 6; k++) {
        texte += `:${m.mycelium_colonie_obstacle(i, k) >>> 0}`;
      }
    }
    return texte;
  };

  /** Vérifie les bords d'un état : au-delà du dernier obstacle ou de la dernière soudure, `NaN` ; un `k` hors de
   * portée aussi. */
  const verifierBords = (nom, nbObstacles, nbAnastomoses) => {
    if (!Number.isNaN(m.mycelium_colonie_obstacle(nbObstacles, 0))) {
      throw new Error(`${nom} : obstacle(${nbObstacles}, 0) n'est pas NaN au-delà du dernier obstacle`);
    }
    if (!Number.isNaN(m.mycelium_colonie_soudure(nbAnastomoses, 0))) {
      throw new Error(`${nom} : soudure(${nbAnastomoses}, 0) n'est pas NaN au-delà de la dernière soudure`);
    }
    if (nbObstacles > 0 && !Number.isNaN(m.mycelium_colonie_obstacle(0, 6))) {
      throw new Error(`${nom} : obstacle(0, 6) n'est pas NaN pour un k hors de portée`);
    }
    if (nbAnastomoses > 0 && !Number.isNaN(m.mycelium_colonie_soudure(0, 3))) {
      throw new Error(`${nom} : soudure(0, 3) n'est pas NaN pour un k hors de portée`);
    }
  };

  /** Écrit la ligne d'état `nom`, puis vérifie ses bords. */
  const lire = (nom) => {
    const temps = m.mycelium_colonie_temps() >>> 0;
    const duree = m.mycelium_colonie_duree() >>> 0;
    const nbSegments = m.mycelium_colonie_nb_segments() >>> 0;
    const pointes = m.mycelium_colonie_pointes() >>> 0;
    const ramifications = m.mycelium_colonie_ramifications() >>> 0;
    const nbAnastomoses = m.mycelium_colonie_anastomoses() >>> 0;
    const nbObstacles = m.mycelium_colonie_nb_obstacles() >>> 0;
    const coordonnees = [];
    for (let i = 0; i < nbSegments; i++) {
      for (let k = 0; k < 6; k++) {
        coordonnees.push(m.mycelium_colonie_segment(i, k));
      }
    }
    const soudures = [];
    for (let i = 0; i < nbAnastomoses; i++) {
      for (let k = 0; k < 3; k++) {
        soudures.push(m.mycelium_colonie_soudure(i, k));
      }
    }
    lignes.push(
      `${nom}=${temps},${duree},${nbSegments},${pointes},${ramifications},${nbAnastomoses},` +
        `${hacher(coordonnees)},${obstacles()},${hacher(soudures)},${empreinte()}`,
    );
    verifierBords(nom, nbObstacles, nbAnastomoses);
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
