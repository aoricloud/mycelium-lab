# Mycelium Lab

Un moteur libre de simulation de croissance mycélienne, en Rust, orienté recherche : un labo virtuel. Il simule la
croissance d'un réseau d'hyphes (extension des apex, ramification, anastomose, transport des nutriments), en tire des
mesures (biomasse, densité, topologie, dimension fractale) et compare des scénarios, avec un rendu qu'on prend
plaisir à regarder.

**Où il en est** : les étapes 0 et 1 de la V0 (voir [FEUILLE-DE-ROUTE.md](FEUILLE-DE-ROUTE.md)) sont faites — la
graine, l'empreinte, l'écrivain PNG et le binaire `mycelium lancer` ; la colonie, le substrat qui diffuse, les mesures
(biomasse, dimension fractale par comptage de boîtes), avec leurs tests d'acceptation et leurs aperçus dans `apercus/`.
Les étapes suivantes — le couplage de la colonie et du substrat, la figure de référence, la vitrine en WebAssembly —
sont dans la feuille de route.

## Ce qui le tient

- Rust sans dépendance ; même scénario et même graine, même état, sous Linux comme sous Windows.
- La preuve est l'état simulé et des nombres, et une figure publiée refaite en nombres ; les images sont pour l'œil.
- Les règles du dépôt sont dans [AGENTS.md](AGENTS.md) ; le modèle, ses équations et ses limites, dans
  [MODELE.md](MODELE.md).

## L'essayer

Sur un clone :

```sh
# la preuve complète : les tests, le lint et le format
cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check

# réécrit les trois aperçus de l'étape 1 dans apercus/
cargo run --example apercus_etape_1

# un scénario de l'étape 0 tient en une ligne, boite 64 48
echo 'boite 64 48' > boite.txt
cargo run -- lancer boite.txt --graine 1 --sortie sortie
```

La dernière commande écrit `empreinte.txt` et `apercu.png` dans `sortie/`.

## Comment il se construit

Mycelium Lab est construit par Kraken, un orchestrateur d'agents : ses mains travaillent sur les issues, chacune sur
une branche `kraken/…`, et GitHub fusionne une PR quand sa CI est verte sous Linux et sous Windows. L'humain qui mène
le dépôt en garde les réglages et regarde après coup.

## Licence

MIT, voir [LICENSE](LICENSE).
