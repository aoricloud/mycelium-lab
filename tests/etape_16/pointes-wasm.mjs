// Lit les trois lectures de l'étape 16 (le pas de naissance et l'hyphe d'un segment, la position d'une pointe vivante)
// par le module WebAssembly de la vitrine.
//
// Usage : node pointes-wasm.mjs <fichier.wasm> <numéro du scénario> <graine> <morceaux>
//
// Comme `densite-wasm.mjs` de l'étape 11 : le script charge le module (sans aucune importation), lit d'abord ce qu'il rend
// avant tout démarrage, choisit le scénario de ce numéro (`mycelium_choisir_scenario` doit rendre 1), démarre avec la
// graine (un entier de 32 bits non signé) et avance par morceaux : <morceaux> est une liste de nombres de pas séparés par
// des virgules (par exemple `1,0,6,13,0,60`), chacun un entier de 32 bits non signé. Il écrit sur la sortie standard, une
// par ligne, `clé=valeur`, dans cet ordre :
//
//   avant          : `mycelium_segment_temps(i)`, `mycelium_segment_apex(i)`, `mycelium_pointe(i, 0)` et
//                    `mycelium_pointe(i, 1)` pour i = 0, 1 et `u32::MAX`, sur le module neuf, séparés par des virgules
//                    (douze `NaN` attendus) ;
//   apres_choix    : les mêmes douze valeurs, relues après le choix du scénario, qui ne démarre rien ;
//   lecture_<i>    : pour chaque état, d'abord le départ (`lecture_depart`, avant tout pas), puis chaque morceau `i` (de 0)
//                    après l'avoir avancé : `temps,segments,pointes,hache_temps,hache_apex,hache_pointes,empreinte` où
//                    `temps` est `mycelium_temps()`, `segments` `mycelium_nb_segments()`, `pointes` `mycelium_pointes()`, les
//                    trois hachés les SHA-256 (64 caractères hexadécimaux minuscules) des f64 petit-boutistes de toutes les
//                    lectures : `mycelium_segment_temps(i)` pour i de 0 à segments - 1 ; `mycelium_segment_apex(i)` de même ;
//                    `mycelium_pointe(j, 0)` puis `mycelium_pointe(j, 1)` pour j de 0 à pointes - 1, en ordre de j ; et
//                    `empreinte` les huit mots de `mycelium_empreinte_mot`, lus **après** ces lectures, en 64 chiffres
//                    hexadécimaux. Aux morceaux impairs les pointes sont lues avant les segments, aux autres après : le
//                    résultat ne doit pas dépendre de l'ordre ;
//   bords_<i>      : 1 si, dans cet état, les trois lectures rendent `NaN` hors de portée (chacune au premier indice de
//                    trop, à 65536, 2^31 et `u32::MAX` ; `k` de 2, 2^31 et `u32::MAX`) et si le dernier segment et la
//                    dernière pointe, quand il y en a, ne rendent pas `NaN` ; sinon 0.
//
// Si le fichier ou les arguments sont faux, ou si une exportation manque, écrit un message sur la sortie d'erreur et sort
// avec le code 1. Aucune dépendance : les modules intégrés de Node seulement.

import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";

const EXPORTATIONS = [
  "mycelium_choisir_scenario",
  "mycelium_demarrer",
  "mycelium_avancer",
  "mycelium_temps",
  "mycelium_nb_segments",
  "mycelium_pointes",
  "mycelium_empreinte_mot",
  "mycelium_segment_temps",
  "mycelium_segment_apex",
  "mycelium_pointe",
];
const MAX_U32 = 4294967295;

/** Lit un entier de 32 bits non signé écrit en chiffres décimaux ; échoue sinon. */
function entier32(texte, nom) {
  if (!/^[0-9]+$/.test(texte) || Number(texte) > MAX_U32) {
    throw new Error(`${nom} doit être un entier de 0 à ${MAX_U32} : « ${texte} »`);
  }
  return Number(texte);
}

/** Le SHA-256 en hexadécimal d'une suite de f64, petit-boutistes, mis bout à bout. */
function hacher(valeurs) {
  const octets = Buffer.alloc(8 * valeurs.length);
  valeurs.forEach((valeur, i) => octets.writeDoubleLE(valeur, 8 * i));
  return createHash("sha256").update(octets).digest("hex");
}

async function principal() {
  const args = process.argv.slice(2);
  if (args.length !== 4) {
    throw new Error("usage : node pointes-wasm.mjs <fichier.wasm> <numéro du scénario> <graine> <morceaux>");
  }
  const numero = entier32(args[1], "le numéro du scénario");
  const graine = entier32(args[2], "la graine");
  const morceaux = args[3].split(",").map((texte) => entier32(texte, "un nombre de pas"));

  const octets = readFileSync(args[0]);
  const { instance } = await WebAssembly.instantiate(octets, {});
  const m = instance.exports;
  for (const nom of EXPORTATIONS) {
    if (typeof m[nom] !== "function") {
      throw new Error(`le module n'exporte pas la fonction ${nom}`);
    }
  }

  const lignes = [];
  /** Les lectures aux indices 0, 1 et u32::MAX, tant que rien n'est démarré. */
  const nanAttendu = () =>
    [0, 1, MAX_U32]
      .flatMap((i) => [m.mycelium_segment_temps(i), m.mycelium_segment_apex(i), m.mycelium_pointe(i, 0), m.mycelium_pointe(i, 1)])
      .join(",");
  lignes.push(`avant=${nanAttendu()}`);

  // Les fonctions qui rendent un entier rendent des i32 : `>>> 0` les relit comme des entiers non signés.
  if (m.mycelium_choisir_scenario(numero) >>> 0 !== 1) {
    throw new Error(`le module refuse le scénario ${numero}`);
  }
  lignes.push(`apres_choix=${nanAttendu()}`);

  m.mycelium_demarrer(graine);

  /** Les empreintes lues en huit mots. */
  const empreinte = () => {
    let texte = "";
    for (let i = 0; i < 8; i++) {
      texte += (m.mycelium_empreinte_mot(i) >>> 0).toString(16).padStart(8, "0");
    }
    return texte;
  };

  /** Une ligne `lecture_<nom>` et une ligne `bords_<nom>` pour l'état courant ; `pointesAvant` pour l'ordre des lectures. */
  const lire = (nom, pointesAvant) => {
    const segments = m.mycelium_nb_segments() >>> 0;
    const pointes = m.mycelium_pointes() >>> 0;
    const lireSegments = () => {
      const temps = [];
      const apex = [];
      for (let i = 0; i < segments; i++) {
        temps.push(m.mycelium_segment_temps(i));
        apex.push(m.mycelium_segment_apex(i));
      }
      return [temps, apex];
    };
    const lirePointes = () => {
      const positions = [];
      for (let j = 0; j < pointes; j++) {
        positions.push(m.mycelium_pointe(j, 0), m.mycelium_pointe(j, 1));
      }
      return positions;
    };
    let temps, apex, positions;
    if (pointesAvant) {
      positions = lirePointes();
      [temps, apex] = lireSegments();
    } else {
      [temps, apex] = lireSegments();
      positions = lirePointes();
    }
    lignes.push(
      `lecture_${nom}=${m.mycelium_temps() >>> 0},${segments},${pointes},${hacher(temps)},${hacher(apex)},${hacher(positions)},${empreinte()}`,
    );
    // Chaque lecture à chaque indice de trop (le premier, 65536, 2^31, u32::MAX) et, pour les pointes, à chaque `k` ;
    // `k` de trop (2, 2^31, u32::MAX) sur la première et la dernière pointe.
    const horsDePortee = [
      ...[segments, 65536, 2147483648, MAX_U32].flatMap((i) => [m.mycelium_segment_temps(i), m.mycelium_segment_apex(i)]),
      ...[pointes, 65536, 2147483648, MAX_U32].flatMap((i) => [m.mycelium_pointe(i, 0), m.mycelium_pointe(i, 1)]),
      ...[0, pointes - 1].flatMap((i) => [2, 2147483648, MAX_U32].map((k) => m.mycelium_pointe(i, k))),
      m.mycelium_pointe(MAX_U32, MAX_U32),
    ].every(Number.isNaN);
    const dedans = [
      segments === 0 || !Number.isNaN(m.mycelium_segment_temps(segments - 1)),
      segments === 0 || !Number.isNaN(m.mycelium_segment_apex(segments - 1)),
      pointes === 0 || (!Number.isNaN(m.mycelium_pointe(pointes - 1, 0)) && !Number.isNaN(m.mycelium_pointe(pointes - 1, 1))),
    ].every(Boolean);
    lignes.push(`bords_${nom}=${horsDePortee && dedans ? 1 : 0}`);
  };

  lire("depart", false);
  morceaux.forEach((pas, i) => {
    m.mycelium_avancer(pas);
    lire(String(i), i % 2 === 1);
  });

  process.stdout.write(lignes.join("\n") + "\n");
}

principal().catch((erreur) => {
  process.stderr.write(`pointes-wasm : ${erreur.message}\n`);
  process.exitCode = 1;
});
