// Lit l'empreinte d'une simulation par le module WebAssembly de la vitrine, comme le fait la page.
//
// Usage : node empreinte-wasm.mjs <fichier.wasm> <graine> <pas>
//
// Charge le module (sans aucune importation), démarre la simulation avec la graine (un entier de 32 bits non
// signé), l'avance de <pas> pas, lit les huit mots de 32 bits de l'empreinte et écrit sur la sortie standard les
// 64 caractères hexadécimaux minuscules et un saut de ligne, rien d'autre. Si le fichier ou les arguments sont
// faux, ou si une exportation manque, écrit un message sur la sortie d'erreur et sort avec le code 1.
//
// Ce script n'a aucune dépendance : les modules intégrés de Node seulement. Il est écrit à part des tests Rust et
// du module : c'est lui qui prouve que le fichier `docs/mycelium.wasm` donne le même résultat que le code natif.

import { readFileSync } from "node:fs";

const EXPORTATIONS = ["mycelium_demarrer", "mycelium_avancer", "mycelium_empreinte_mot"];
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
  if (args.length !== 3) {
    throw new Error("usage : node empreinte-wasm.mjs <fichier.wasm> <graine> <pas>");
  }
  const graine = entier32(args[1], "la graine");
  const pas = entier32(args[2], "le nombre de pas");

  const octets = readFileSync(args[0]);
  const { instance } = await WebAssembly.instantiate(octets, {});
  const exportations = instance.exports;
  for (const nom of EXPORTATIONS) {
    if (typeof exportations[nom] !== "function") {
      throw new Error(`le module n'exporte pas la fonction ${nom}`);
    }
  }

  exportations.mycelium_demarrer(graine);
  exportations.mycelium_avancer(pas);
  let empreinte = "";
  for (let i = 0; i < 8; i++) {
    // Les fonctions rendent un i32 : `>>> 0` le relit comme un entier non signé.
    empreinte += (exportations.mycelium_empreinte_mot(i) >>> 0).toString(16).padStart(8, "0");
  }
  process.stdout.write(empreinte + "\n");
}

principal().catch((erreur) => {
  process.stderr.write(`empreinte-wasm : ${erreur.message}\n`);
  process.exitCode = 1;
});
