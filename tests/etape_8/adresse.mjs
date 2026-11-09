// Fait tourner les fonctions pures de `docs/adresse.js` (l'adresse de la page : quel scénario, quelle graine).
//
// Usage : node adresse.mjs <dossier docs> <dossier de travail>   (les cas se lisent sur l'entrée standard)
//
// Le script copie `adresse.js` dans un dossier neuf, créé sous le dossier de travail, à côté d'un `package.json` qui
// déclare des modules : un fichier `.js` ne s'importe comme module que dans un dossier qui le dit, et la page, elle,
// est servie comme module par son `<script type="module">`. Il importe la copie, répond à un cas par ligne d'entrée
// et efface le dossier neuf. Chaque ligne d'entrée est une commande suivie de ses arguments, séparés par des
// tabulations ; chaque ligne de sortie est la réponse à la ligne d'entrée de même rang :
//
//   lire     <fragment> <graine par défaut>  ->  « <scénario> » tabulation « <graine> »   (lireAdresse)
//   ecrire   <nom> <graine>                  ->  l'adresse écrite                          (ecrireAdresse)
//   numero   <nom>                           ->  le numéro du scénario, ou -1              (numeroDuScenario)
//   graine   <texte>                         ->  la graine lue, ou « null »                (lireGraine)
//   noms                                     ->  les noms, dans l'ordre des numéros, séparés par des virgules
//   max                                      ->  la plus grande graine
//
// Si une commande est inconnue ou si une exportation manque, écrit un message sur la sortie d'erreur et sort avec le
// code 1. Aucune dépendance : les modules intégrés de Node seulement.

import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

const EXPORTATIONS = ["lireAdresse", "ecrireAdresse", "numeroDuScenario", "lireGraine", "NOMS_SCENARIOS", "GRAINE_MAX"];

async function principal() {
  const args = process.argv.slice(2);
  if (args.length !== 2) {
    throw new Error("usage : node adresse.mjs <dossier docs> <dossier de travail>");
  }
  mkdirSync(args[1], { recursive: true });
  const dossier = mkdtempSync(join(args[1], "adresse-"));
  try {
    writeFileSync(join(dossier, "package.json"), '{"type":"module"}\n');
    copyFileSync(join(args[0], "adresse.js"), join(dossier, "adresse.js"));
    const adresse = await import(pathToFileURL(join(dossier, "adresse.js")).href);
    for (const nom of EXPORTATIONS) {
      if (!(nom in adresse)) {
        throw new Error(`adresse.js n'exporte pas ${nom}`);
      }
    }

    const sortie = [];
    const entree = readFileSync(0, "utf8");
    for (const ligne of entree.split("\n")) {
      if (ligne === "") {
        continue;
      }
      const [commande, ...arguments_] = ligne.split("\t");
      if (commande === "lire") {
        const lue = adresse.lireAdresse(arguments_[0], Number(arguments_[1]));
        sortie.push(`${lue.scenario}\t${lue.graine}`);
      } else if (commande === "ecrire") {
        sortie.push(String(adresse.ecrireAdresse(arguments_[0], Number(arguments_[1]))));
      } else if (commande === "numero") {
        sortie.push(String(adresse.numeroDuScenario(arguments_[0])));
      } else if (commande === "graine") {
        sortie.push(String(adresse.lireGraine(arguments_[0])));
      } else if (commande === "noms") {
        sortie.push(adresse.NOMS_SCENARIOS.join(","));
      } else if (commande === "max") {
        sortie.push(String(adresse.GRAINE_MAX));
      } else {
        throw new Error(`commande inconnue : « ${commande} »`);
      }
    }
    process.stdout.write(sortie.join("\n") + "\n");
  } finally {
    rmSync(dossier, { recursive: true, force: true });
  }
}

principal().catch((erreur) => {
  process.stderr.write(`adresse : ${erreur.message}\n`);
  process.exitCode = 1;
});
