# Mycelium Lab : les règles du dépôt

Ce dépôt est mené par Lydite : des mains de Lydite et des sessions de l'humain y travaillent, et aujourd'hui GitHub
fusionne seul une PR verte d'une branche `lydite/` dont le corps dit que Lydite a rejoué sa preuve sur sa tête. Le plan
est dans `FEUILLE-DE-ROUTE.md` ; le modèle, ses équations et ses limites, dans `MODELE.md`.

## La preuve

`cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check`, verts sous Linux et sous Windows.

## Les règles

1. **Même graine, même colonie** : au bit près sous Linux, sous Windows et dans la page.

2. **Le moteur n'emploie que des calculs exacts partout** : `+`, `-`, `*`, `/`, `sqrt`, `abs`, `floor`, `round`,
   `min`, `max`, `clamp` et `as`. Toute autre fonction passe par un seul fichier, copié d'une source publiée et comparé
   sur les trois cibles (le module de la page se reconstruit à la main : la CI n'a pas la cible wasm).

3. **Un ordre fixe** : aucune table parcourue dans un ordre qui change (ni `HashMap` ni `HashSet`), ni fil ni horloge
   dans la simulation. Un seul test lit tout le code pour le vérifier : `tests/regles_du_code.rs`.

4. **Aucune dépendance Cargo**. Un code copié garde sa licence et dit d'où il vient.

5. **La page peut employer une bibliothèque copiée dans le dépôt**, version, licence et empreinte notées et vérifiées
   par un test ; rien ne se charge d'un autre site. La page 2D marche toujours sans la 3D.

6. **La preuve, c'est l'état et les nombres** : une empreinte par scénario et par graine, des mesures avec une
   tolérance. Un changement voulu de l'état met à jour son empreinte dans la même PR et le dit.

7. **Les aperçus de `apercus/` sont pour l'œil** ; chaque PR qui change ce qu'on voit en montre un.

8. **Un mécanisme cite sa référence publiée quand elle existe** ; sinon il dit « choix » ou « dérivé ».

9. **Ce qui porte un nombre** : les tests d'abord, le code ensuite, en deux vagues. Le reste : tests et code ensemble.

10. **Critique** : `src/`, `references/`, les tests d'empreinte, `scenarios/`, `Cargo.toml`, `Cargo.lock`,
    `rust-toolchain.toml`, tout `build.rs`, `docs/`, `AGENTS.md` et `CLAUDE.md`. Une tâche qui les touche porte
    `critique : oui`. Leur fusion : selon le choix de l'humain.

11. **Rien sous `.github/`**, aucun `.cargo/` : la CI et la fusion restent à l'humain. Le banc ne prédit pas la
    résistance d'un vrai matériau.
