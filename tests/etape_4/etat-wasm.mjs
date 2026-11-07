// Lit l'état d'une simulation par les accesseurs du module WebAssembly de la vitrine, comme le fait la page.
//
// Usage : node etat-wasm.mjs <fichier.wasm> <graine> <pas>
//
// Complète `empreinte-wasm.mjs`, qui ne lit que l'empreinte de l'état interne : la page, elle, ne lit que les
// accesseurs (temps, boîte, segments, substrat, biomasse…). Ce script charge le module (sans aucune importation),
// démarre la simulation avec la graine (un entier de 32 bits non signé), l'avance de <pas> pas, puis écrit sur la
// sortie standard, une par ligne, `clé=valeur`, dans cet ordre :
//
//   temps, largeur, hauteur, segments, pointes, anastomoses  : les entiers, en décimal ;
//   biomasse                                                 : les 8 octets du f64, petit-boutiste, en hexadécimal ;
//   hache_segments  : le SHA-256 (64 caractères hexadécimaux minuscules) des f64 petit-boutistes x0, y0, x1, y1 de
//                     chaque segment, du premier au dernier ;
//   hache_substrat  : le SHA-256 des f64 petit-boutistes des cases, ligne par ligne (y de 0 à hauteur - 1, et pour
//                     chaque y, x de 0 à largeur - 1) ;
//   bords           : 1 si les accesseurs rendent NaN pour le segment `segments`, pour la coordonnée 4, pour la case
//                     (largeur, 0) et pour la case (0, hauteur), et 0 pour le mot d'empreinte 8 ; sinon 0.
//
// Si le fichier ou les arguments sont faux, ou si une exportation manque, écrit un message sur la sortie d'erreur et
// sort avec le code 1. Aucune dépendance : les modules intégrés de Node seulement.

import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";

const EXPORTATIONS = [
  "mycelium_demarrer",
  "mycelium_avancer",
  "mycelium_temps",
  "mycelium_largeur",
  "mycelium_hauteur",
  "mycelium_nb_segments",
  "mycelium_segment",
  "mycelium_substrat",
  "mycelium_biomasse",
  "mycelium_pointes",
  "mycelium_anastomoses",
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

/** Le SHA-256 en hexadécimal d'un tampon. */
function sha256(tampon) {
  return createHash("sha256").update(tampon).digest("hex");
}

async function principal() {
  const args = process.argv.slice(2);
  if (args.length !== 3) {
    throw new Error("usage : node etat-wasm.mjs <fichier.wasm> <graine> <pas>");
  }
  const graine = entier32(args[1], "la graine");
  const pas = entier32(args[2], "le nombre de pas");

  const octets = readFileSync(args[0]);
  const { instance } = await WebAssembly.instantiate(octets, {});
  const m = instance.exports;
  for (const nom of EXPORTATIONS) {
    if (typeof m[nom] !== "function") {
      throw new Error(`le module n'exporte pas la fonction ${nom}`);
    }
  }

  m.mycelium_demarrer(graine);
  m.mycelium_avancer(pas);

  // Les fonctions rendent des i32 : `>>> 0` les relit comme des entiers non signés.
  const temps = m.mycelium_temps() >>> 0;
  const largeur = m.mycelium_largeur() >>> 0;
  const hauteur = m.mycelium_hauteur() >>> 0;
  const nb = m.mycelium_nb_segments() >>> 0;

  const biomasse = Buffer.alloc(8);
  biomasse.writeDoubleLE(m.mycelium_biomasse());

  const segments = Buffer.alloc(32 * nb);
  for (let i = 0; i < nb; i++) {
    for (let k = 0; k < 4; k++) {
      segments.writeDoubleLE(m.mycelium_segment(i, k), 32 * i + 8 * k);
    }
  }

  const substrat = Buffer.alloc(8 * largeur * hauteur);
  for (let y = 0; y < hauteur; y++) {
    for (let x = 0; x < largeur; x++) {
      substrat.writeDoubleLE(m.mycelium_substrat(x, y), 8 * (y * largeur + x));
    }
  }

  const bords =
    Number.isNaN(m.mycelium_segment(nb, 0)) &&
    Number.isNaN(m.mycelium_segment(0, 4)) &&
    Number.isNaN(m.mycelium_substrat(largeur, 0)) &&
    Number.isNaN(m.mycelium_substrat(0, hauteur)) &&
    m.mycelium_empreinte_mot(8) === 0;

  const lignes = [
    `temps=${temps}`,
    `largeur=${largeur}`,
    `hauteur=${hauteur}`,
    `segments=${nb}`,
    `pointes=${m.mycelium_pointes() >>> 0}`,
    `anastomoses=${m.mycelium_anastomoses() >>> 0}`,
    `biomasse=${biomasse.toString("hex")}`,
    `hache_segments=${sha256(segments)}`,
    `hache_substrat=${sha256(substrat)}`,
    `bords=${bords ? 1 : 0}`,
  ];
  process.stdout.write(lignes.join("\n") + "\n");
}

principal().catch((erreur) => {
  process.stderr.write(`etat-wasm : ${erreur.message}\n`);
  process.exitCode = 1;
});
