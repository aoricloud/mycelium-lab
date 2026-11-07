# Mycelium Lab

Un moteur libre de simulation de croissance mycélienne, en Rust, orienté recherche : un labo virtuel. Il simule la
croissance d'un réseau d'hyphes (extension des apex, ramification, anastomose, transport des nutriments), en tire des
mesures (biomasse, densité, topologie, dimension fractale) et compare des scénarios, avec un rendu qu'on prend
plaisir à regarder.

**Où il en est** : rien ne tourne encore. La V0, une colonie qui pousse en 2D sur une boîte de Petri depuis une spore,
est décrite dans [FEUILLE-DE-ROUTE.md](FEUILLE-DE-ROUTE.md).

## Ce qui le tient

- Rust sans dépendance ; même scénario et même graine, même état, sous Linux comme sous Windows.
- La preuve est l'état simulé et des nombres, et une figure publiée refaite en nombres ; les images sont pour l'œil.
- Les règles du dépôt sont dans [AGENTS.md](AGENTS.md) ; le modèle, ses équations et ses limites, dans
  [MODELE.md](MODELE.md).

## Comment il se construit

Mycelium Lab est construit par Kraken, un orchestrateur d'agents : ses mains travaillent sur les issues, chacune sur
une branche `kraken/…`, et GitHub fusionne une PR quand sa CI est verte sous Linux et sous Windows. L'humain qui mène
le dépôt en garde les réglages et regarde après coup.

## Licence

MIT, voir [LICENSE](LICENSE).
