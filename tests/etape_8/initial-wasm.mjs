// Lit ce que fait le module WebAssembly de la vitrine quand il vient d'être instancié et dans ses premiers choix.
//
// Usage : node initial-wasm.mjs <fichier.wasm>
//
// Les tests natifs ne peuvent pas voir ce que valent les fonctions avant tout démarrage : l'état global du processus
// est déjà posé par les autres tests. Un module neuf, lui, n'a rien démarré. Ce script instancie le module (sans
// aucune importation) deux fois, et écrit sur la sortie standard une ligne `clé=valeur` par grandeur lue, dans
// cet ordre.
//
// Première instance, qui ne démarre rien d'abord :
//   avant_duree, avant_temps, avant_largeur, avant_obstacles : ce que rendent `mycelium_duree`, `mycelium_temps`,
//                         `mycelium_largeur` et `mycelium_nb_obstacles`, en décimal ;
//   avant_obstacle      : `mycelium_obstacle(0, k)` pour k de 0 à 3, séparés par des virgules ;
//   avant_obstacle_hors : `mycelium_obstacle(4294967295, 0)` puis `mycelium_obstacle(0, 4)`, séparés par une virgule ;
//   choix_0_a_5         : ce que rend `mycelium_choisir_scenario` pour 0, 1, 2, 3, 4 puis 5, séparés par des
//                         virgules ;
//   choix_refuses       : ce qu'il rend pour 6, 7, 8, 255, 256, 260, 65540, 2147483652 et 4294967295 ;
//   apres_choix_duree, apres_choix_temps, apres_choix_largeur, apres_choix_obstacles : comme `avant_…`, après ces
//                         choix, toujours sans démarrage (choisir ne démarre rien) ;
// puis, la simulation démarrée par `mycelium_demarrer(1)` (le dernier choix accepté est 5, les refus n'ont rien
// changé), les trois lignes `garde_duree`, `garde_largeur` et `garde_obstacles` ; puis, après `mycelium_choisir_scenario(0)`
// et 5 pas (choisir n'agit pas sur la simulation en cours), `en_cours_duree`, `en_cours_largeur`, `en_cours_obstacles`
// et `en_cours_temps` ; puis, après un nouveau `mycelium_demarrer(1)`, `suivant_duree`, `suivant_largeur`,
// `suivant_obstacles` et `suivant_temps`.
//
// Seconde instance, qui ne choisit rien : après `mycelium_demarrer(1)`, `defaut_duree`, `defaut_largeur` et
// `defaut_obstacles` ; puis, après 80 pas, `defaut_empreinte` (les 64 caractères hexadécimaux).
//
// Si le fichier ou l'argument est faux, ou si une exportation manque, écrit un message sur la sortie d'erreur et sort
// avec le code 1. Aucune dépendance : les modules intégrés de Node seulement.

import { readFileSync } from "node:fs";

const EXPORTATIONS = [
  "mycelium_choisir_scenario",
  "mycelium_demarrer",
  "mycelium_avancer",
  "mycelium_duree",
  "mycelium_temps",
  "mycelium_largeur",
  "mycelium_nb_obstacles",
  "mycelium_obstacle",
  "mycelium_empreinte_mot",
];

async function instancier(octets) {
  const { instance } = await WebAssembly.instantiate(octets, {});
  for (const nom of EXPORTATIONS) {
    if (typeof instance.exports[nom] !== "function") {
      throw new Error(`le module n'exporte pas la fonction ${nom}`);
    }
  }
  return instance.exports;
}

/** Les fonctions rendent des i32 : `>>> 0` les relit comme des entiers non signés. */
const nu32 = (valeur) => valeur >>> 0;

async function principal() {
  const args = process.argv.slice(2);
  if (args.length !== 1) {
    throw new Error("usage : node initial-wasm.mjs <fichier.wasm>");
  }
  const octets = readFileSync(args[0]);
  const lignes = [];

  const m = await instancier(octets);
  const etat = (prefixe) => {
    lignes.push(`${prefixe}_duree=${nu32(m.mycelium_duree())}`);
    lignes.push(`${prefixe}_temps=${nu32(m.mycelium_temps())}`);
    lignes.push(`${prefixe}_largeur=${nu32(m.mycelium_largeur())}`);
    lignes.push(`${prefixe}_obstacles=${nu32(m.mycelium_nb_obstacles())}`);
  };

  etat("avant");
  lignes.push(`avant_obstacle=${[0, 1, 2, 3].map((k) => m.mycelium_obstacle(0, k)).join(",")}`);
  lignes.push(`avant_obstacle_hors=${m.mycelium_obstacle(4294967295, 0)},${m.mycelium_obstacle(0, 4)}`);
  lignes.push(`choix_0_a_5=${[0, 1, 2, 3, 4, 5].map((n) => nu32(m.mycelium_choisir_scenario(n))).join(",")}`);
  const refuses = [6, 7, 8, 255, 256, 260, 65540, 2147483652, 4294967295];
  lignes.push(`choix_refuses=${refuses.map((n) => nu32(m.mycelium_choisir_scenario(n))).join(",")}`);
  etat("apres_choix");

  m.mycelium_demarrer(1);
  lignes.push(`garde_duree=${nu32(m.mycelium_duree())}`);
  lignes.push(`garde_largeur=${nu32(m.mycelium_largeur())}`);
  lignes.push(`garde_obstacles=${nu32(m.mycelium_nb_obstacles())}`);

  m.mycelium_choisir_scenario(0);
  m.mycelium_avancer(5);
  lignes.push(`en_cours_duree=${nu32(m.mycelium_duree())}`);
  lignes.push(`en_cours_largeur=${nu32(m.mycelium_largeur())}`);
  lignes.push(`en_cours_obstacles=${nu32(m.mycelium_nb_obstacles())}`);
  lignes.push(`en_cours_temps=${nu32(m.mycelium_temps())}`);

  m.mycelium_demarrer(1);
  lignes.push(`suivant_duree=${nu32(m.mycelium_duree())}`);
  lignes.push(`suivant_largeur=${nu32(m.mycelium_largeur())}`);
  lignes.push(`suivant_obstacles=${nu32(m.mycelium_nb_obstacles())}`);
  lignes.push(`suivant_temps=${nu32(m.mycelium_temps())}`);

  // Une seconde instance, qui ne choisit rien : le scénario du départ est le couplage.
  const n = await instancier(octets);
  n.mycelium_demarrer(1);
  lignes.push(`defaut_duree=${nu32(n.mycelium_duree())}`);
  lignes.push(`defaut_largeur=${nu32(n.mycelium_largeur())}`);
  lignes.push(`defaut_obstacles=${nu32(n.mycelium_nb_obstacles())}`);
  n.mycelium_avancer(80);
  let empreinte = "";
  for (let i = 0; i < 8; i++) {
    empreinte += nu32(n.mycelium_empreinte_mot(i)).toString(16).padStart(8, "0");
  }
  lignes.push(`defaut_empreinte=${empreinte}`);

  process.stdout.write(lignes.join("\n") + "\n");
}

principal().catch((erreur) => {
  process.stderr.write(`initial-wasm : ${erreur.message}\n`);
  process.exitCode = 1;
});
