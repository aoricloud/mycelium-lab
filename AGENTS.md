# Mycelium Lab : les règles du dépôt

Ce dépôt est mené par Kraken : des mains (des agents de toute sorte) y travaillent sur des issues, chacune sur une
branche `kraken/…`, et GitHub fusionne une PR quand sa CI est verte sous Linux et sous Windows. Ces règles valent pour
toute main. Le plan est dans `FEUILLE-DE-ROUTE.md` ; le modèle, ses équations et ses limites, dans `MODELE.md`.

## La preuve

`cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check`, verte sous Linux et sous Windows.

## Les règles

1. **Sans dépendance** : rien dans `[dependencies]`, `[dev-dependencies]` ni `[build-dependencies]`. Le générateur
   pseudo-aléatoire, l'écrivain PNG et l'empreinte s'écrivent dans le dépôt.
2. **Déterministe sur toutes les machines** : même scénario et même graine, même état, sous Linux comme sous Windows.
   - ni `sin`, `cos`, `tan`, `exp`, `ln`, `powf`, ni aucune autre fonction de `f64` qui passe par la bibliothèque
     mathématique du système : des tables, ou des versions écrites dans le dépôt ; `+`, `-`, `*`, `/` et `sqrt`
     restent permis ;
   - aucune `HashMap` ni aucun `HashSet` parcourus : des `Vec` ou des `BTreeMap` ;
   - un ordre de mise à jour fixe, sans horloge ni fil d'exécution dans la simulation.
3. **La preuve, c'est l'état et les nombres** : l'empreinte de l'état simulé, par scénario et par graine
   (`references/<scénario>.empreinte`), et des mesures avec une tolérance. Les aperçus de `apercus/` sont pour l'œil :
   les embellir ne change aucune référence.
4. **Un changement voulu de l'état** met à jour son empreinte de référence dans la même PR, et la PR le dit.
5. **Les tests d'acceptation d'une étape** s'écrivent avant son code, par une vague à eux ; la vague du code ne touche
   pas à leurs fichiers (`FEUILLE-DE-ROUTE.md`, « Les tests avant les mains »).
6. **Chaque mécanisme arrive avec sa référence publiée** (un article, une figure ou une équation), nommée dans
   `MODELE.md`.
7. **Rien sous `.github/`** : la CI et la fusion restent à l'humain. **Aucun `.cargo/`** : la CI le refuse, car sa
   configuration peut changer ce que `cargo test` joue.

Rien n'est critique dans ce dépôt.
