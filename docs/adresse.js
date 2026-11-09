/*
  L'adresse de la vitrine : quel scénario, quelle graine.

  Ce petit module ne touche ni à la page ni au module WebAssembly : ce ne sont que des fonctions pures sur du
  texte, pour que `app.js` s'en serve et qu'on puisse les faire tourner seules, sous Node.

  L'adresse s'écrit `#scenario=<nom>&graine=<n>`. Le nom est l'un de ceux de NOMS_SCENARIOS, et sa position dans
  cette liste est le numéro que le module WebAssembly attend (mycelium_choisir_scenario) : 0 pour « libre » (le
  fichier couplage.txt), 1 pour « terrain », 2 pour « tache », 3 pour « tache-sans », 4 pour « mur », 5 pour
  « mur-sans ». Cet ordre est fixe.
*/

// Les noms des six scénarios, dans l'ordre de leurs numéros.
export const NOMS_SCENARIOS = ["libre", "terrain", "tache", "tache-sans", "mur", "mur-sans"];

// Le scénario qu'on montre quand l'adresse n'en nomme pas un qu'on connaît.
export const SCENARIO_PAR_DEFAUT = "libre";

// La plus grande graine : un entier de 32 bits non signé, ce que le module accepte.
export const GRAINE_MAX = 4294967295;

// Le numéro du scénario de ce nom pour le module, ou -1 si le nom n'est pas celui d'un scénario. La recherche se
// fait dans la liste (et non dans un objet) pour qu'un nom comme « constructor » ou « __proto__ » ne passe pas.
export function numeroDuScenario(nom) {
  return NOMS_SCENARIOS.indexOf(nom);
}

// Lit un texte en graine : rend l'entier de 0 à GRAINE_MAX qu'il écrit en chiffres décimaux, ou null si ce n'en est
// pas un (vide, signe, point, espace, lettres, notation en e, ou trop grand).
export function lireGraine(texte) {
  if (typeof texte !== "string" || !/^[0-9]+$/.test(texte)) {
    return null;
  }
  const valeur = Number(texte);
  if (!Number.isSafeInteger(valeur) || valeur > GRAINE_MAX) {
    return null;
  }
  return valeur;
}

// Les paires `clé=valeur` d'un fragment d'adresse (avec ou sans le `#` du début), dans l'ordre. Une clé qui
// revient garde sa première valeur ; une paire sans `=` a une valeur vide.
function paires(fragment) {
  const texte = typeof fragment === "string" ? fragment.replace(/^#/, "") : "";
  const resultat = new Map();
  for (const morceau of texte.split("&")) {
    const egal = morceau.indexOf("=");
    const cle = egal === -1 ? morceau : morceau.slice(0, egal);
    const valeur = egal === -1 ? "" : morceau.slice(egal + 1);
    if (!resultat.has(cle)) {
      resultat.set(cle, valeur);
    }
  }
  return resultat;
}

// Lit l'adresse (le `location.hash`) : rend { scenario, graine }. Un nom absent ou inconnu donne « libre » ; une
// graine absente ou invalide donne `graineParDefaut` (celle du champ de la page).
export function lireAdresse(fragment, graineParDefaut) {
  const lues = paires(fragment);
  const nom = lues.get("scenario");
  const scenario = numeroDuScenario(nom) === -1 ? SCENARIO_PAR_DEFAUT : nom;
  const graine = lireGraine(lues.get("graine"));
  return { scenario, graine: graine === null ? graineParDefaut : graine };
}

// Écrit l'adresse d'un scénario et d'une graine : `#scenario=<nom>&graine=<n>`.
export function ecrireAdresse(scenario, graine) {
  return `#scenario=${scenario}&graine=${graine}`;
}
