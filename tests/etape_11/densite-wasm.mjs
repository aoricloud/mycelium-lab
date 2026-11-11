// Lit la carte de densité d'une simulation par les accesseurs du module WebAssembly de la vitrine.
//
// Usage : node densite-wasm.mjs <fichier.wasm> <numéro du scénario> <graine> <morceaux>
//
// Complète `etat-wasm.mjs` de l'étape 8 : la page, quand la case « Densité » est cochée, ne lit que `mycelium_densite`
// (une case) et `mycelium_densite_max`. Ce script charge le module (sans aucune importation) et lit d'abord ce qu'il
// rend avant tout démarrage ; puis il choisit le scénario de ce numéro (`mycelium_choisir_scenario` doit rendre 1),
// démarre la simulation avec la graine (un entier de 32 bits non signé) et l'avance par morceaux : <morceaux> est une
// liste de nombres de pas séparés par des virgules (par exemple `1,0,6,13,0,60`), chacun un entier de 32 bits non
// signé. Il écrit sur la sortie standard, une par ligne, `clé=valeur`, dans cet ordre :
//
//   avant_densite        : `mycelium_densite(x, y)` pour (0, 0), (80, 80) et (4294967295, 4294967295) sur le module neuf,
//                          séparés par des virgules (`NaN` attendu) ;
//   avant_max            : `mycelium_densite_max()` sur le module neuf (`NaN` attendu) ;
//   apres_choix          : les quatre valeurs précédentes, relues après le choix du scénario, qui ne démarre rien ;
//   apres_demarrage      : `maximum,hache` de la carte qui suit le démarrage (lus comme pour `carte_<i>`, ci-dessous) : une
//                          carte vide, dont le maximum vaut 0.0 ;
//   carte_<i>            : pour chaque morceau `i` (de 0), après l'avoir avancé : `temps,maximum,occupees,hache` où `temps` est
//                          `mycelium_temps()`, `maximum` les 8 octets de `mycelium_densite_max()` en hexadécimal
//                          petit-boutiste, `occupees` le nombre de cases de densité strictement positive et `hache` le
//                          SHA-256 (64 caractères hexadécimaux minuscules) des f64 petit-boutistes de toutes les cases, ligne
//                          par ligne (y de 0 à hauteur - 1, et pour chaque y, x de 0 à largeur - 1). Aux morceaux impairs le
//                          maximum est lu avant les cases, aux autres après : la carte ne doit pas dépendre de l'ordre ;
//   bords                : 1 si, après le dernier morceau, `mycelium_densite` rend `NaN` pour les cases (largeur, 0),
//                          (largeur, hauteur - 1), (0, hauteur), (largeur - 1, hauteur), (largeur, hauteur), (4294967295, 0),
//                          (0, 4294967295) et (0, y) où y est le plus petit entier dont le produit par `largeur` dépasse
//                          4294967295 (en WebAssembly 32 bits, `y * largeur` reviendrait alors sur une case de la carte si
//                          `y` n'était pas contrôlé) ; sinon 0 ;
//   apres_relance        : `maximum,hache` de la carte lue juste après un nouveau `mycelium_demarrer` (même graine), qui suit une
//                          carte lue : une carte vide, dont le maximum vaut 0.0.
//
// Si le fichier ou les arguments sont faux, ou si une exportation manque, écrit un message sur la sortie d'erreur et
// sort avec le code 1. Aucune dépendance : les modules intégrés de Node seulement.

import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";

const EXPORTATIONS = [
  "mycelium_choisir_scenario",
  "mycelium_demarrer",
  "mycelium_avancer",
  "mycelium_temps",
  "mycelium_largeur",
  "mycelium_hauteur",
  "mycelium_densite",
  "mycelium_densite_max",
];
const MAX_U32 = 4294967295;

/** Lit un entier de 32 bits non signé écrit en chiffres décimaux ; échoue sinon. */
function entier32(texte, nom) {
  if (!/^[0-9]+$/.test(texte) || Number(texte) > MAX_U32) {
    throw new Error(`${nom} doit être un entier de 0 à ${MAX_U32} : « ${texte} »`);
  }
  return Number(texte);
}

/** Les 8 octets d'un f64, petit-boutiste, en hexadécimal. */
function hexadecimal(valeur) {
  const octets = Buffer.alloc(8);
  octets.writeDoubleLE(valeur);
  return octets.toString("hex");
}

/** Le SHA-256 en hexadécimal d'un tampon. */
function hacher(tampon) {
  return createHash("sha256").update(tampon).digest("hex");
}

async function principal() {
  const args = process.argv.slice(2);
  if (args.length !== 4) {
    throw new Error("usage : node densite-wasm.mjs <fichier.wasm> <numéro du scénario> <graine> <morceaux>");
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
  /** Ce que rendent les deux accesseurs sur trois cases et au maximum, tant que rien n'est démarré. */
  const nanAttendu = () =>
    [m.mycelium_densite(0, 0), m.mycelium_densite(80, 80), m.mycelium_densite(MAX_U32, MAX_U32)].join(",");
  lignes.push(`avant_densite=${nanAttendu()}`);
  lignes.push(`avant_max=${m.mycelium_densite_max()}`);

  // Les fonctions rendent des i32 : `>>> 0` les relit comme des entiers non signés.
  if (m.mycelium_choisir_scenario(numero) >>> 0 !== 1) {
    throw new Error(`le module refuse le scénario ${numero}`);
  }
  lignes.push(`apres_choix=${nanAttendu()},${m.mycelium_densite_max()}`);

  m.mycelium_demarrer(graine);
  const largeur = m.mycelium_largeur() >>> 0;
  const hauteur = m.mycelium_hauteur() >>> 0;

  /** La carte entière : ses octets (pour la hacher) et le nombre de cases de densité strictement positive. */
  const lireLaCarte = () => {
    const cases = Buffer.alloc(8 * largeur * hauteur);
    let occupees = 0;
    for (let y = 0; y < hauteur; y++) {
      for (let x = 0; x < largeur; x++) {
        const densite = m.mycelium_densite(x, y);
        cases.writeDoubleLE(densite, 8 * (y * largeur + x));
        if (densite > 0) {
          occupees += 1;
        }
      }
    }
    return { cases, occupees };
  };

  const vide = lireLaCarte();
  lignes.push(`apres_demarrage=${hexadecimal(m.mycelium_densite_max())},${hacher(vide.cases)}`);

  morceaux.forEach((pas, i) => {
    m.mycelium_avancer(pas);
    const maximumAvant = i % 2 === 1 ? m.mycelium_densite_max() : null;
    const { cases, occupees } = lireLaCarte();
    const maximum = maximumAvant ?? m.mycelium_densite_max();
    lignes.push(`carte_${i}=${m.mycelium_temps() >>> 0},${hexadecimal(maximum)},${occupees},${hacher(cases)}`);
  });

  const bords = [
    [largeur, 0],
    [largeur, hauteur - 1],
    [0, hauteur],
    [largeur - 1, hauteur],
    [largeur, hauteur],
    [MAX_U32, 0],
    [0, MAX_U32],
    [0, Math.floor(MAX_U32 / largeur) + 1],
  ].every(([x, y]) => Number.isNaN(m.mycelium_densite(x, y)));
  lignes.push(`bords=${bords ? 1 : 0}`);

  // Relancer remet la carte à zéro, même quand la précédente a été lue.
  m.mycelium_demarrer(graine);
  lignes.push(`apres_relance=${hexadecimal(m.mycelium_densite_max())},${hacher(lireLaCarte().cases)}`);

  process.stdout.write(lignes.join("\n") + "\n");
}

principal().catch((erreur) => {
  process.stderr.write(`densite-wasm : ${erreur.message}\n`);
  process.exitCode = 1;
});
