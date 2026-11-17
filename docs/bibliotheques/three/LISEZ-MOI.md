# three.js, copié dans le dépôt

La colonie en 3D se dessinera avec three.js (choix de l'humain, le 17/11/2026). Rien ne se charge d'un autre site :
la bibliothèque est copiée ici, telle quelle, et `tests/bibliotheques.rs` vérifie chaque fichier octet pour octet.

- **Version** : 0.185.1, la dernière publiée sur npm avec ses fichiers minifiés (les versions 0.186.0 et suivantes
  ne les publient plus).
- **Source** : le paquet npm `three` 0.185.1, `https://registry.npmjs.org/three/-/three-0.185.1.tgz`, d'intégrité
  `sha512-5aojFCXKwnjBRZvUnt3WFfEcvUJgkN5LlijRFN95hMy8WVkG4I0QNcJE+OuWvuJ0bOdStrbfXn0pkd6/QyiAlg==`.
- **Licence** : MIT, copiée dans `LICENSE`.
- **Fichiers**, copiés sans changement depuis le paquet, avec leur SHA-256 :

| Fichier | Dans le paquet | Octets | SHA-256 |
|---|---|---|---|
| `three.module.min.js` | `build/three.module.min.js` | 365 552 | `86bcee248b64f44bcfc23c331ae74619061957d59cab040171dcb6fb5900beb6` |
| `three.core.min.js` | `build/three.core.min.js` | 385 386 | `05b2609338c76cd65daf74f3ac515bc9a5045e1b3b33edc07d8c9bd55250fa90` |
| `LICENSE` | `LICENSE` | 1 081 | `8b378ebe60e2fe500158cb0ac71cb5e8b7d92953c2abcc63a0eb90499653b5bc` |

`three.module.min.js` importe `./three.core.min.js`, et rien d'autre. Changer de version, c'est remplacer les trois
fichiers, ce tableau et les empreintes de `tests/bibliotheques.rs` dans la même PR.
