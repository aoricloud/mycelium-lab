// Lit l'empreinte d'une simulation par le module WebAssembly de la vitrine, pour un scénario choisi.
//
// Usage : node empreinte-wasm.mjs <fichier.wasm> <numéro du scénario> <graine> <pas>
//
// Comme `tests/etape_4/empreinte-wasm.mjs`, avec le choix du scénario en plus : charge le module (sans aucune
// importation), choisit le scénario de ce numéro par `mycelium_choisir_scenario` (qui doit rendre 1), démarre la
// simulation avec la graine (un entier de 32 bits non signé), l'avance de <pas> pas, lit les huit mots de 32 bits de
// l'empreinte et écrit sur la sortie standard les 64 caractères hexadécimaux minuscules et un saut de ligne, rien
// d'autre. Si le fichier ou les arguments sont faux, ou si une exportation manque, écrit un message sur la sortie
// d'erreur et sort avec le code 1.
//
// Aucune dépendance : les modules intégrés de Node seulement.

import { readFileSync } from "node:fs";

const EXPORTATIONS = [
  "mycelium_choisir_scenario",
  "mycelium_demarrer",
  "mycelium_avancer",
  "mycelium_empreinte_mot",
];
const MAX_U32 = 4294967295;

/** Lit un entier de 32 bits non signé écrit en chiffres décimaux ; échoue sinon. */
function entier32(texte, nom) {
  if (!/^[0-9]+$/.test(texte) || Number(texte) > MAX_U32) {
    throw new Error(`${nom} doit être un entier de 0 à ${MAX_U32} : « ${texte} »`);
  }
  return Number(texte);
}

async function principal() {
  const args = process.argv.slice(2);
  if (args.length !== 4) {
    throw new Error("usage : node empreinte-wasm.mjs <fichier.wasm> <numéro du scénario> <graine> <pas>");
  }
  const numero = entier32(args[1], "le numéro du scénario");
  const graine = entier32(args[2], "la graine");
  const pas = entier32(args[3], "le nombre de pas");

  const octets = readFileSync(args[0]);
  const { instance } = await WebAssembly.instantiate(octets, {});
  const exportations = instance.exports;
  for (const nom of EXPORTATIONS) {
    if (typeof exportations[nom] !== "function") {
      throw new Error(`le module n'exporte pas la fonction ${nom}`);
    }
  }

  // Les fonctions rendent des i32 : `>>> 0` les relit comme des entiers non signés.
  if (exportations.mycelium_choisir_scenario(numero) >>> 0 !== 1) {
    throw new Error(`le module refuse le scénario ${numero}`);
  }
  exportations.mycelium_demarrer(graine);
  exportations.mycelium_avancer(pas);
  let empreinte = "";
  for (let i = 0; i < 8; i++) {
    empreinte += (exportations.mycelium_empreinte_mot(i) >>> 0).toString(16).padStart(8, "0");
  }
  process.stdout.write(empreinte + "\n");
}

principal().catch((erreur) => {
  process.stderr.write(`empreinte-wasm : ${erreur.message}\n`);
  process.exitCode = 1;
});
