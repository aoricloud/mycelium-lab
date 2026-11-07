# Le modèle de l'étape 3 : la figure 3 de Meškauskas, Fricker et Moore (2004)

Ce document décrit le modèle tel que `src/voisinage.rs` l'implémente, et les nombres que
`examples/figure_3.rs` mesure. Il est écrit pour être lu sans connaître Kraken : les noms de champs du code sont
donnés à côté des symboles, et chaque valeur est reliée à sa source.

**En bref.** Une colonie 2D d'apex pousse depuis une spore ; chaque pointe lit un champ de densité et un vecteur
d'évitement produits par les hyphes déjà déposées, tourne son cap, avance d'un pas et se ramifie là où la densité
est faible. Sur les graines 1 à 30, pendant 400 pas, avec les paramètres publiés de la figure 3, la pente moyenne du
rayon sur `]200, 400]` vaut **0,927989445** (écart-type 0,045114820 ; minimum 0,827648054 ; maximum 0,999753857), le
R² moyen de l'ajustement linéaire vaut **0,998992171**, l'exposant moyen vaut **1,012551264**, et le nombre de pointes
à 400 pas vaut **192,333** en moyenne (182 à 208). La valeur publiée est **0,86** ; la pente mesurée n'est pas 0,86 :
l'écart est **+0,067989445**. La figure est donc reproduite **en esprit** (croissance linéaire, pente sous 1, exposant
proche de 1), mais **pas en valeur**.

## 1. Les deux articles de référence

| Article | Adresse bibliographique | Adresse en ligne | Ce qu'on y prend |
|---|---|---|---|
| [A] Meškauskas, Fricker et Moore (2004) | *Mycological Research* **108**(11):1241–1256 | réimpression : <https://davidmoore.org.uk/cyberweb/images_2016/Manuscripts/Meskauskas_etal_simulating_coloniesNS02.pdf> | la figure 3 et sa pente d'environ 0,86 ; la vitesse d'un apex (1), le facteur de persistance `k = 0,1`, le seuil de densité `0,06`, la constante `lc = 20`, la ramification à 40 % par itération ; aucune équation n'est prise dans [A] |
| [B] Meškauskas, McNulty et Moore (2004) | *Mycological Research* **108**:341–353 | adresse en ligne non consignée dans le dépôt | les équations **3 et 4** (champ de densité et poids des sections), l'équation **11** (vecteur d'évitement), l'équation **13** (mise à jour du cap) |

L'article [B] est le compagnon mathématique de [A]. Les numéros d'équations sont ceux cités par la feuille de route et
par la documentation de `src/voisinage.rs` ; les PDF n'ont pas été relus ligne à ligne dans cet environnement (voir
« Ce qui n'a pas été vérifié »). L'article [A] est en 3D ; c'est une de ses figures seulement qui se refait en 2D (voir
« Limites »).

La prédiction de vitesse radiale constante, utilisée comme seconde vérification, vient d'Edelstein (1982), *Journal of
Theoretical Biology* **98** ; la feuille de route cite à côté d'elle le lien <https://arxiv.org/pdf/q-bio/0402004>.

## 2. Le modèle

### 2.1 Notation

Le temps est entier : `t = 0, 1, 2, …`. À chaque pas :

- `x_i(t)` est la position de la pointe de l'apex `i` au début du pas `t` ;
- `h_i(t)` est son cap, un vecteur unitaire (`h_i(t) = (hx_i, hy_i)`, `|h_i| = 1`) ;
- `σ_i(t)` est sa section courante (voir § 2.7) ;
- `S(t)` est l'ensemble des segments d'hyphe déjà déposés avant le pas `t` ;
- pour un segment `s`, `σ(s)` est sa section, `n(σ(s))` le nombre de segments de cette section au moment de la
  lecture, `d_s(x)` la distance euclidienne du point `x` au segment fini `s`, et `q_s(x)` le point de `s` le plus
  proche de `x` ;
- `[condition]` vaut 1 si la condition est vraie, 0 sinon.

Les paramètres sont notés `pas`, `persistance` (`k`), `seuil_densite` (`D_seuil`), `probabilite_ramification` (`p`),
`lc`, `epsilon` (`ε`), `portee` et `angle_max` (`α_max`).

### 2.2 Le champ de densité (équations 3 et 4)

Chaque segment déjà déposé contribue au champ de densité lu par une pointe. Un segment d'une section de `n` segments
pèse `lc / n` ; sa contribution décroît comme `1 / (d² + ε²)` et s'annule au-delà de `portee` :

```text
D_i(t) = Σ_{s ∈ S(t), σ(s) ≠ σ_i(t)}
          [d_s(x_i(t)) ≤ portee] · (lc / n(σ(s))) · 1 / (d_s(x_i(t))² + ε²)
```

Le point important pour un biologiste : le champ est **la somme des hyphes voisines** ; chaque section y contribue
pour un poids total constant `lc`, réparti sur ses segments (`lc / n` par segment) ; il est donc plus fort dans un
réseau dense. Deux adoucissements sont ceux du code :

- `ε` (`epsilon = 1,0`) évite la divergence quand une pointe passe à distance nulle d'un segment ;
- `portee` (`60,0`) est une coupure : les segments plus loin ne comptent pas. L'article n'a pas cette coupure ; c'est
  un choix de calcul, sans effet notable sur le champ à longue distance puisque la contribution décroît en `1/d²`.

La section courante de la pointe est exclue : **une pointe ne se repousse pas elle-même**. Les internodes plus anciens
de la même hyphe, eux, sont des voisins comme les autres. `n(σ(s))` est le nombre de segments de la section au moment
de la lecture, pas au moment où le segment a été déposé : le poids d'un segment diminue à mesure que l'internode
s'allonge.

### 2.3 Le vecteur d'évitement (équation 11)

Chaque segment repousse la pointe dans la direction qui va de son point le plus proche vers la pointe, avec le même
poids de section et un noyau plus raide :

```text
E_i(t) = Σ_{s ∈ S(t), σ(s) ≠ σ_i(t)}
          [d_s(x_i(t)) ≤ portee] · (lc / n(σ(s))) · (x_i(t) − q_s(x_i(t))) / (d_s(x_i(t))² + ε²)³
```

Le poids scalaire contient déjà un facteur `1 / (d² + ε²)`, donc il décroît en `1/d⁶` à grande distance ; la norme
du vecteur, elle, décroît en `1/d⁵`, car le numérateur contient en plus `(x_i − q_s)`, de longueur `d`. Ce sont les
segments très proches qui dominent. `E_i` est un vecteur ; sa norme n'est pas utilisée telle quelle, puisque
l'équation suivante le normalise.

### 2.4 Le cap (équation 13)

Le nouveau cap mélange l'ancien cap et la direction d'évitement :

```text
h_i'(t) = ( k · h_i(t) + (1 − k) · E_i(t) / ‖E_i(t)‖ )
          / ‖ k · h_i(t) + (1 − k) · E_i(t) / ‖E_i(t)‖ ‖
```

avec `k = persistance = 0,1`. Trois cas particuliers sont traités par le code :

- si `k = 1`, l'autotropisme est supprimé et `h_i' = h_i` ;
- si `E_i = 0` (pointe isolée, aucun segment dans la portée), le cap est inchangé ;
- si la combinaison donne un vecteur nul, le cap est inchangé.

Le cap reste donc unitaire. Il n'est pas limité à un ensemble fini de directions : la combinaison et la normalisation
sont continues. Les tables de cosinus et de sinus à 360 entrées ne servent qu'à tourner un cap d'un angle entier lors
d'une ramification (§ 2.6).

### 2.5 L'avancée

Chaque apex avance de `pas` dans son nouveau cap et laisse un segment :

```text
x_i(t + 1) = x_i(t) + pas · h_i'(t)
```

Le segment va de `x_i(t)` à `x_i(t + 1)`. La longueur d'un segment vaut donc exactement `pas` (à l'arrondi près des
opérations `f64`). Avec `pas = 1,0`, une pente de rayon se lit directement en fois la vitesse d'un apex.

### 2.6 La ramification

Au début du pas, la pointe a lu sa densité `D_i(t)`. La ramification se produit si deux conditions sont réunies :

1. un tirage uniforme `U ∈ [0, 1)` est inférieur à `p = probabilite_ramification = 0,4` ;
2. `D_i(t) < D_seuil = 0,06`.

Alors :

```text
côté  ∈ {−1, +1}                       (équiprobable)
α     ∈ {0, 1, …, α_max}               (uniforme, en degrés entiers ; α_max = 180)
h_fille = R(côté · α) · h_i'(t)         (rotation du nouveau cap du parent)
x_fille(t)     = x_i(t)                 (le point de ramification est la position du parent au début du pas)
x_fille(t + 1) = x_i(t) + pas · h_fille
```

La branche naît donc **au point où le parent se trouvait au début du pas**, et elle avance dès ce pas de naissance :
elle laisse son premier segment en même temps que le parent. Elle ne peut pas se ramifier avant le pas suivant. Avec
`angle_max = 180`, l'angle est tiré uniformément entre 0 et 180 degrés, sans préférence de côté.

Détail important pour la structure : quand une ramification a lieu, **le parent et la branche commencent chacun une
section neuve** au point de ramification. Le segment que le parent vient de déposer de ce point vers l'avant est
rattaché à la nouvelle section du parent ; l'ancienne section s'arrête au point de ramification.

### 2.7 Les sections

Une **section** est un internode : la portion d'hyphe entre deux ramifications. Elle porte un numéro, et chaque
segment déposé est rattaché à la section de l'apex qui l'a déposé.

- Le poids d'un segment dans le champ est `lc / n`, où `n` est le nombre de segments de sa section au moment de la
  lecture.
- La section courante d'une pointe est exclue de son propre champ : une pointe ne voit ni le segment qu'elle vient de
  déposer, ni les autres segments du même internode.
- Les internodes plus anciens de la même hyphe (avant la dernière ramification) ne sont pas exclus : ils comptent
  comme des voisins.
- À chaque ramification, deux sections neuves apparaissent (celle de la branche et celle de la continuation du
  parent), et l'ancienne section du parent se termine au point de ramification.

### 2.8 L'ordre du pas et la mise à jour synchrone

Un pas se déroule ainsi, dans un ordre fixe :

1. **Perception.** Tous les apex présents au début du pas lisent le même état : le champ de densité et le vecteur
   d'évitement calculés sur `S(t)`, avant qu'aucun segment du pas ne soit déposé. Les caps et les densités sont
   mémorisés.
2. **Avancée.** Chaque apex du début du pas avance avec son cap mémorisé et dépose son segment.
3. **Ramifications.** Les branches éventuelles naissent au point de départ du parent ; elles avancent dans le même pas
   et déposent leur premier segment.
4. **Enregistrement.** Les ramifications, le rayon et le nombre de pointes du pas sont ajoutés aux séries.

La mise à jour est donc **synchrone** : un apex ne lit jamais l'état déjà avancé d'un autre apex du même pas. Les
tirages aléatoires, eux, sont consommés dans l'ordre fixe des identifiants des apex.

### 2.9 Le hasard et le déterminisme

Le générateur est `splitmix64`, écrit dans `src/alea.rs` ; la graine est un entier `u64`. Le même scénario et la même
graine donnent la même suite de tirages, donc le même état, sur toutes les machines. Aucune fonction de la
bibliothèque mathématique du système n'est appelée : `+`, `-`, `*`, `/` et `sqrt` suffisent, et le logarithme des
mesures est celui, écrit dans le dépôt, de `mesures::logarithme`. Les cosinus et sinus des angles de ramification
viennent des tables `croissance::COS` et `croissance::SIN` (un degré par indice).

L'empreinte de référence de la graine 7 après 150 pas est dans
`references/voisinage-graine-7.empreinte` :

```text
45186a1be013f720b5e41bd5b32df64e07fb32c531143b818a5c31519e925180
```

Détail d'implémentation sans effet sur les nombres : les segments sont rangés dans un index spatial par case de côté
`min(portee, 8)` ; une requête visite les cases autour de la pointe jusqu'à `portee + pas/2`. Cela garantit de voir
tous les segments à moins de `portee` de la pointe (le milieu d'un segment est au plus à `pas/2` de son point le plus
proche), puis la coupure exacte est appliquée. La distance utilisée est celle au segment fini (projection bornée à
`[0, 1]`), pas à la droite qui le porte.

## 3. Les paramètres et leur provenance

Les valeurs de `Parametres::figure_3()` sont les suivantes. La colonne « provenance » distingue ce qui est **publié
dans l'article**, ce qui est **choisi par nous** et ce qui serait **réglé** au sens d'un ajustement sur des données.

| Champ du code | Symbole | Valeur | Unité | Provenance | Source ou raison |
|---|---|---:|---|---|---|
| `pas` | `pas` | 1,0 | longueur / pas | publié dans l'article | vitesse d'un apex (1) [A] |
| `persistance` | `k` | 0,1 | sans dimension | publié dans l'article | facteur de persistance `k = 0,1` [A] |
| `seuil_densite` | `D_seuil` | 0,06 | densité | publié dans l'article | seuil de densité 0,06 [A] |
| `probabilite_ramification` | `p` | 0,4 | par pas | publié dans l'article | 40 % par itération [A] |
| `lc` | `lc` | 20,0 | densité · longueur² | publié dans l'article | constante d'ajustement `lc = 20` du poids d'une section [A] |
| `epsilon` | `ε` | 1,0 | longueur | choisi par nous | adoucissement du noyau, absent des valeurs publiées ; évite la divergence à distance nulle |
| `portee` | `portee` | 60,0 | longueur | choisi par nous | coupure du champ ; l'article n'a pas de coupure |
| `angle_max` | `α_max` | 180 | degré | choisi par nous | l'article ne donne pas l'angle de ramification ; 180° = tirage uniforme sans préférence |
| `nb_apex_depart` | `n_0` | 1 | nombre | choisi par nous | l'article ne donne pas le nombre d'apex au départ ; une spore |

**Aucun paramètre n'est réglé au sens d'un ajustement.** Le dépôt ne contient ni calibration sur des colonies réelles,
ni recherche de la valeur 0,86. Les valeurs non publiées sont des choix de commodité, pas des valeurs ajustées. C'est
pourquoi la pente mesurée peut différer de la valeur publiée sans que cela soit un échec de réglage.

### 3.1 Les conventions de mesure

| Convention | Valeur | Provenance |
|---|---|---|
| Spore | `(0, 0)` | choix de l'exemple ; le modèle est invariant par translation |
| Durée | 400 pas | tests d'acceptation de l'étape 3 |
| Fenêtre de la pente et de l'exposant | `]200, 400]`, soit `t = 201, …, 400` (200 points) | tests d'acceptation |
| Graines de la mesure | 1 à 30 (jeu A) | tests d'acceptation |
| Rayon | plus grande distance entre la spore et une extrémité de segment | code (`Colonie::rayon`) |
| Ajustement linéaire | moindres carrés de `r(t)` sur `t` ; pente et R² | code (`voisinage::ajuster_rayon`) |
| Exposant | moindres carrés de `ln r(t)` sur `ln t` | code (`voisinage::exposant`) |
| Nombre de pointes | nombre d'apex vivants après 400 pas | code (`Colonie::pointes`) |

## 4. Les hypothèses

Les hypothèses ci-dessous sont celles du module `voisinage` (la figure 3). Elles ne décrivent pas le modèle couplé de
l'étape 2, qui a son propre substrat.

### 4.1 Substrat riche

Le module ne lit aucun champ de nutriments, ne consomme rien et ne modélise aucune carence : le milieu est supposé
riche et homogène. La seule limite à l'extension est la répulsion des hyphes déjà déposées et la règle de
ramification. Le couplage colonie–substrat (`src/simulation.rs`, étape 2) est un autre modèle ; il ne sert pas à la
figure 3.

### 4.2 Apex immortels

Aucun apex ne meurt, ne s'anastomose et ne fusionne. Le nombre de pointes ne fait qu'augmenter : avec un seul apex au
départ, `pointes(t) = 1 + ramifications(t)`. Il n'y a ni compétition entre apex, ni arrêt de croissance, ni
dégénérescence.

### 4.3 Sections

Chaque internode entre deux ramifications est une section. Le poids d'un segment est `lc / n`, où `n` est le nombre de
segments de sa section au moment de la lecture. La section courante d'une pointe est exclue de son propre champ : une
pointe ne se repousse pas elle-même, mais les internodes plus anciens de la même hyphe sont des voisins. Une
ramification crée deux sections neuves et termine l'ancienne section du parent au point de ramification.

### 4.4 Mise à jour synchrone

Tous les apex présents au début d'un pas lisent le même état, puis avancent avec leurs caps mémorisés. Un apex ne voit
jamais les segments déposés pendant le même pas par un autre apex. Les apex nés pendant le pas avancent dans ce pas,
mais ne se ramifient qu'au pas suivant.

### 4.5 Noyau en 1/d² en 2D

Le champ de densité est une somme de noyaux isotropes `1 / (d² + ε²)` dans le plan. À grande distance, il décroît
comme `1/d²`. C'est le noyau en `1/d²` du modèle publié, conservé en 2D avec l'adoucissement `ε` et la coupure
`portee` ; ce n'est pas la solution d'une équation de diffusion 2D, et le modèle ne prétend pas en dériver. Le vecteur
d'évitement utilise le même noyau élevé au cube : son poids scalaire décroît en `1/d⁶`, mais la norme du vecteur
décroît en `1/d⁵`, car le numérateur contient en plus `(x_i − q_s)`, de longueur `d`.

### 4.6 Autres hypothèses de fait

- Le plan est infini : il n'y a ni boîte de Petri, ni bord, ni paroi dans ce module.
- Les positions sont continues ; un segment a exactement la longueur `pas`.
- Le cap est unitaire et continu ; seuls les angles de ramification sont entiers (tables à 1 degré).
- La ramification est un processus de Bernoulli par apex et par pas, indépendant entre apex, conditionné par la
  densité et par le tirage.
- Le côté de la branche est équiprobable ; l'angle est uniforme dans `0 … angle_max`.
- Le hasard est entièrement déterministe à graine fixée (`splitmix64`).

## 5. Le rayon, la fenêtre et les statistiques

### 5.1 Définition du rayon

Le rayon `r(t)` est la **plus grande distance entre la spore et une extrémité de segment** parmi tous les segments
déposés jusqu'au pas `t` inclus. Ce n'est pas la distance maximale des pointes vivantes, ni le rayon du plus petit
cercle contenant la colonie : c'est un maximum courant sur les extrémités déjà atteintes. Il est donc monotone (il ne
peut pas diminuer) et vaut 0 au temps 0. Avec la spore en `(0, 0)` :

```text
r(t) = max_{s déposé au plus tard au pas t} ‖ (x1_s, y1_s) − (0, 0) ‖
```

où `(x1_s, y1_s)` est l'extrémité d'arrivée du segment `s`.

### 5.2 La fenêtre de mesure

La fenêtre est `]200, 400]` : elle exclut le pas 200 et inclut le pas 400, soit les temps `t = 201, 202, …, 400`
(200 points). Les 200 premiers pas sont donc exclus : c'est une convention des tests d'acceptation, destinée à laisser
la colonie établir sa structure et à mesurer la croissance tardive. La fenêtre n'est pas donnée par l'article.

Sur ces points, on ajuste une droite `r(t) = a · t + b` par moindres carrés. La **pente** `a` est en unités de
longueur par pas ; avec `pas = 1,0`, elle s'interprète directement comme un multiple de la vitesse d'un apex. Le
**R²** est `1 − SCR / SCT`, où `SCR` est la somme des carrés des résidus et `SCT` la somme des carrés des écarts à la
moyenne de `r`.

### 5.3 L'exposant

L'exposant `β` est la pente de la régression de `ln r(t)` sur `ln t`, sur la même fenêtre `]200, 400]`. Il mesure la
forme de la croissance : `β = 1` pour une droite passant par l'origine, `β = 2` pour une croissance en surface
(`r ~ t²`). Le logarithme est celui du dépôt (`mesures::logarithme`), pour que le résultat soit identique sur toutes
les machines.

## 6. Les résultats mesurés

`cargo run --example figure_3` joue les graines 1 à 30, depuis `(0, 0)`, avec `Parametres::figure_3()`, pendant
400 pas. Voici sa sortie, recopiée telle quelle (point décimal, neuf décimales pour les pentes, R² et exposants).

| Graine | Pente du rayon | R² linéaire | Exposant | Pointes à 400 pas |
|---:|---:|---:|---:|---:|
| 1 | 0.941144792 | 0.999323335 | 1.041858772 | 183 |
| 2 | 0.954531480 | 0.999502612 | 1.028748719 | 191 |
| 3 | 0.961668681 | 0.999716165 | 1.049207513 | 196 |
| 4 | 0.870327035 | 0.999331120 | 0.969814801 | 188 |
| 5 | 0.909077082 | 0.997833983 | 0.976529219 | 192 |
| 6 | 0.827648054 | 0.997561752 | 0.912889002 | 188 |
| 7 | 0.995512188 | 0.999999234 | 1.056942282 | 198 |
| 8 | 0.922380028 | 0.996733357 | 1.004922763 | 191 |
| 9 | 0.904093645 | 0.999065589 | 0.999735029 | 188 |
| 10 | 0.932534508 | 0.998913341 | 1.022403102 | 188 |
| 11 | 0.956515190 | 0.999551667 | 1.073423505 | 208 |
| 12 | 0.911407157 | 0.999648949 | 0.988863849 | 194 |
| 13 | 0.932957708 | 0.998477613 | 1.040785544 | 204 |
| 14 | 0.858502269 | 0.999078815 | 0.944759668 | 199 |
| 15 | 0.873955987 | 0.997199919 | 0.939246290 | 191 |
| 16 | 0.876303181 | 0.999117387 | 1.001608091 | 198 |
| 17 | 0.903606018 | 0.999235551 | 0.981388807 | 194 |
| 18 | 0.951403256 | 0.998432395 | 1.042892896 | 193 |
| 19 | 0.977800103 | 0.999980873 | 1.045688145 | 201 |
| 20 | 0.933625945 | 0.998755962 | 1.037865917 | 196 |
| 21 | 0.954182756 | 0.999837100 | 1.054398680 | 195 |
| 22 | 0.994888339 | 0.999999020 | 1.063646975 | 183 |
| 23 | 0.894556532 | 0.999222502 | 0.991456801 | 191 |
| 24 | 0.898512128 | 0.998932450 | 0.969054182 | 185 |
| 25 | 0.974471241 | 0.999459423 | 1.075694179 | 194 |
| 26 | 0.999012156 | 0.999999964 | 1.057056506 | 182 |
| 27 | 0.876974668 | 0.997686799 | 0.969976657 | 196 |
| 28 | 0.963680448 | 0.998483604 | 1.063925656 | 184 |
| 29 | 0.999753857 | 0.999999998 | 1.006011170 | 184 |
| 30 | 0.888656919 | 0.998684654 | 0.965743210 | 195 |

Statistiques du jeu :

| Grandeur | Valeur |
|---|---:|
| Pente moyenne | 0.927989445 |
| Écart-type (population) de la pente | 0.045114820 |
| Pente minimale | 0.827648054 |
| Pente maximale | 0.999753857 |
| R² moyen de l'ajustement linéaire | 0.998992171 |
| Exposant moyen | 1.012551264 |
| Pointes à 400 pas : moyenne | 192.333 |
| Pointes à 400 pas : minimum | 182 |
| Pointes à 400 pas : maximum | 208 |

Toutes les graines ont une pente strictement inférieure à 1 et un R² supérieur à 0,995 ; tous les exposants sont
dans `[0,85 ; 1,15]`.

## 7. La figure est-elle reproduite ?

### 7.1 En esprit : oui

- **Croissance linéaire.** Le R² moyen de l'ajustement linéaire vaut 0,998992171 ; le plus faible d'une graine vaut
  0,996733357 (graine 8). C'est au-dessus du seuil de 0,995 de la feuille de route.
- **Pente sous 1.** La pente moyenne vaut 0,927989445, et la plus grande pente d'une graine vaut 0,999753857 : aucune
  colonie ne dépasse la vitesse de ses apex.
- **Pente dans l'intervalle cible.** La moyenne est dans `[0,80 ; 0,97]`.
- **Exposant proche de 1.** L'exposant moyen vaut 1,012551264 ; les valeurs par graine vont de 0,912889002 à
  1,075694179.
- **Pas de croissance exponentielle.** Les tests d'acceptation vérifient que la pente de `ln(pointes)` sur `]300, 400]`
  est au plus 0,85 fois celle sur `]200, 300]` ; le nombre de pointes à 400 pas reste de l'ordre de 200, pas
  exponentiel.

### 7.2 En valeur : non

La valeur publiée est **0,86** (« the slope of the line in both cases in Fig. 3 is about 0.86 »). La pente moyenne
mesurée sur les 30 graines est **0,927989445**, et l'écart à la valeur publiée est **+0,067989445**. Ce n'est pas
0,86 : la figure n'est pas reproduite en valeur. Aucune graine ne donne exactement 0,86 ; la plus proche est la graine
14, avec 0,858502269 (soit −0,001497731 par rapport à 0,86), et la graine 4 donne 0,870327035 (+0,010327035).

Le commentaire de `tests/etape_3_figure.rs` cite 0,915 pour un prototype ; ce n'est pas la mesure du code actuel. Les
nombres de ce document sont ceux de `examples/figure_3.rs` sur le code commité.

## 8. Les seuils des tests et d'où ils viennent

Les seuils numériques sont, sauf mention contraire, des choix de la vague qui a écrit les tests d'acceptation avant le
code. Ils ne sont pas des valeurs publiées et ils ne sont pas ajustés sur la figure. La feuille de route fixe la
cible de la figure 3 : croissance linéaire (R² ≥ 0,995), pente moyenne sous 1 et dans `[0,80 ; 0,97]` ; la valeur
publiée 0,86 est dans l'intervalle mais n'est pas la cible.

### 8.1 `tests/etape_3_figure.rs`

| Critère | Seuil | Origine |
|---|---|---|
| R² de l'ajustement linéaire, par graine | ≥ 0,995 | feuille de route (cible) |
| Pente du rayon, par graine | 0,80 à 1,00 | feuille de route (pente sous 1) et marge du test |
| Pente moyenne du jeu | 0,80 à 0,97 | feuille de route |
| Exposant, par graine | 0,85 à 1,15 | test d'acceptation (tolérance autour de 1) |
| R² linéaire moyen − R² logarithmique moyen | ≥ 0,004 | test d'acceptation (le linéaire doit mieux ajuster qu'une exponentielle, `ln r` ajusté sur `t`) |
| Écart des pentes moyennes entre les jeux A et B | ≤ 0,03 | test d'acceptation (indépendance à la graine) |
| Écart-type des pentes | ≤ 0,06 | test d'acceptation |
| Étendue des pentes (max − min) | ≤ 0,25 | test d'acceptation |
| Rayon au temps `t` | ≤ `pas · t` | conséquence géométrique : un apex avance d'au plus `pas` par pas |

### 8.2 `tests/etape_3_edelstein.rs`

Ces tests vérifient la prédiction d'Edelstein (1982) : la vitesse radiale d'une colonie est constante. Les tolérances
sont des choix de la vague de tests.

| Critère | Seuil | Origine |
|---|---|---|
| Rapport max/min des pentes moyennes sur `]100, 200]`, `]200, 300]`, `]300, 400]` | ≤ 1,06 | vitesse radiale constante (Edelstein 1982) ; tolérance du test |
| Coefficient de variation des 16 vitesses glissantes (fenêtres de 50 pas, pas de 10, sur `]200, 400]`) | ≤ 0,15 | même prédiction ; tolérance du test |
| Rapport de la moyenne de la seconde moitié à celle de la première moitié des vitesses glissantes | 0,95 à 1,08 | même prédiction ; tolérance du test |
| Exposant moyen du rayon sur `]200, 400]` | 0,95 à 1,05 | même prédiction ; tolérance du test |
| Pente de `ln(pointes)` sur `]300, 400]` | ≤ 0,85 × la pente sur `]200, 300]` | test d'acceptation : pas de croissance exponentielle |

### 8.3 `tests/etape_3_sanity.rs`

| Critère | Seuil | Origine |
|---|---|---|
| Pente du rayon sans autotropisme (`persistance = 1`), par graine | ≥ 0,999 | attendu exact : les apex vont tout droit |
| Moyenne sans autotropisme − moyenne du modèle de base | ≥ 0,05 | test d'acceptation |
| Pointes à 24 pas sans limite de densité (`seuil_densite = ∞`) | ≥ 100 | test d'acceptation ; observé jusqu'à 8 613 pointes |
| Pente de `ln(pointes)` sur `t = 6 … 24` sans limite | ≥ 0,25 | test d'acceptation |
| Pointes à 24 pas du modèle de base | ≤ 10 | test d'acceptation |
| Pente de `ln(pointes)` sur `t = 6 … 24` du modèle de base | ≤ 0,10 | test d'acceptation |
| Densité à chaque ramification du modèle de base | < 0,06 | définition du seuil de densité |
| Densité maximale à une ramification sans limite | > 100 | test d'acceptation |

### 8.4 `tests/etape_3_coherence.rs` et `tests/etape_3_determinisme.rs`

| Critère | Seuil | Origine |
|---|---|---|
| Pointes à 400 pas, graines 1 à 10 | 150 à 260 | test d'acceptation ; observé 181 à 213 dans le commentaire |
| Exposant `β` de `pointes(t) ~ t^β`, `t = 201 … 400` | 1,6 à 2,2 | test d'acceptation ; observé 1,91 |
| Contrats de panique (`depuis_spore`, mesures) | exacts | documentation des fonctions |
| Même graine, mêmes octets et même empreinte | exact | règles du dépôt |
| `avancer_n(n)` = `n` appels à `avancer` | exact | règles du dépôt |
| Empreinte de référence de la graine 7 à 150 pas | exacte | `references/voisinage-graine-7.empreinte` |

## 9. Les limites

### 9.1 2D contre 3D

Le modèle publié est **en 3D**. Ses colonies « de boîte de Petri » sont une couche mince de 3D (quatre unités
d'épaisseur) maintenue par un tropisme horizontal (constante 0,002), pas un vrai 2D. Ce dépôt simule un plan 2D : les
positions, les distances et les noyaux sont dans le plan. La géométrie, la normalisation du champ et les valeurs absolues de densité
ne sont donc pas celles de l'article. Seule la figure 3 est visée en 2D ; les figures 6 et 7, qui dépendent du passage
du 3D au plat, ne sont pas reproduites. Le présent document ne prétend pas que le modèle 2D soit une réduction exacte
du modèle 3D.

### 9.2 Ce que la figure 3 publiée ne donne pas

L'article n'a aucun tableau, et la plupart de ses figures sont des images. La figure 3 donne une droite dont la pente
est « about 0.86 », mais elle ne donne pas :

- les valeurs numériques du rayon, pas à pas, ni les points de la courbe ;
- le nombre de colonies ou de graines simulées, ni la dispersion autour de la droite ;
- la définition exacte du rayon et la fenêtre d'ajustement utilisée pour obtenir 0,86 ;
- les conditions des « deux cas » de la figure 3 (le dépôt n'en identifie pas les paramètres) ;
- l'angle de ramification et le nombre d'apex au départ, qui manquent aussi dans l'article ;
- une coupure du noyau : l'article n'en a pas, alors que le code utilise `portee = 60`.

On ne peut donc pas comparer point par point la courbe publiée et la nôtre, ni calculer un résidu sur les données
d'origine. On compare une statistique (pente, R², exposant) à une valeur annoncée, ce qui est plus faible qu'une
superposition de courbes. C'est pourquoi le document dit « reproduite en esprit, pas en valeur ».

### 9.3 La dérive de la pente au-delà de la durée mesurée

La mesure s'arrête à 400 pas. À l'intérieur de la fenêtre, les tests d'Edelstein bornent la dérive : le rapport entre
les pentes moyennes des fenêtres `]100, 200]`, `]200, 300]` et `]300, 400]` doit rester au plus 1,06. Mais **rien
n'est mesuré après 400 pas**. Le modèle n'affirme donc pas que la vitesse radiale reste constante indéfiniment : à
mesure que la colonie s'épaissit et que les sections s'allongent, la densité et les poids `lc / n` évoluent, et la
pente pourrait dériver. Aucune extrapolation au-delà de la durée mesurée n'est faite, et aucune affirmation de régime
asymptotique n'est avancée.

### 9.4 Autres limites

- Le module n'a **ni substrat, ni carence, ni anastomose, ni mort d'apex, ni bord de boîte**. Ce sont des hypothèses
  du modèle de la figure 3, pas des oublis : le modèle couplé de l'étape 2 est distinct.
- Le plan est infini ; une colonie qui pousserait longtemps sortirait de toute boîte de Petri réelle sans rencontrer
  de paroi.
- La ramification est un processus de Bernoulli par pas ; elle ne modélise pas de mécanisme biochimique explicite.
- Le modèle ne représente pas la largeur des hyphes, ni leur diamètre, ni la fusion, ni la translocation.
- Les paramètres non publiés (`epsilon`, `portee`, `angle_max`, `nb_apex_depart`) sont des choix ; une autre valeur
  changerait les nombres, même si les tests qualitatifs resteraient sans doute satisfaits.
- La pente de 0,86 est une valeur annoncée pour la figure 3 de l'article, pas une mesure de colonies réelles ; le
  dépôt ne contient aucune calibration sur des données biologiques.

## 10. Ce qui n'a pas été vérifié

- **Les PDF des deux articles n'ont pas été relus ligne à ligne** dans cet environnement (pas d'accès réseau). Les
  numéros d'équations (3, 4, 11, 13) viennent de la feuille de route et de la documentation du module ; les formes
  exactes des équations publiées n'ont pas été confrontées au texte.
- **La courbe publiée de la figure 3 n'a pas été numérisée** : on ne vérifie pas les résidus par rapport à ses points,
  seulement la pente annoncée et la linéarité.
- **La 3D n'a pas été simulée** : le modèle 2D n'est pas comparé à une version 3D du même code, et les figures 6 et 7
  de l'article ne sont pas reproduites.
- **La dérive de la pente après 400 pas n'a pas été mesurée** ; aucune durée plus longue n'a été testée.
- **La décomposition du déficit de pente** (pourquoi la pente vaut environ 0,93 au lieu de 1) n'a pas été mesurée :
  on ne quantifie pas séparément la composante radiale moyenne des caps, l'effet des sections et l'effet de la
  ramification.
- **La sensibilité aux paramètres non publiés** n'a été testée que qualitativement (les tests font varier `lc`,
  `epsilon` et `portee` à l'extrême) ; il n'y a pas de balayage systématique ni d'intervalle de confiance sur la
  pente moyenne.
- **Les 30 graines de l'exemple ne donnent pas un intervalle de confiance** : l'écart-type et le minimum/maximum sont
  descriptifs. Les tests d'acceptation utilisent aussi un second jeu (graines 1001 à 1030) pour vérifier
  l'indépendance à la graine, mais l'exemple ne mesure que le jeu A.
- **Aucune comparaison à des colonies réelles** n'est faite dans ce module ; les mesures de vraies colonies (par
  exemple la dimension fractale d'*Ashbya gossypii*) sont des garde-fous d'autres étapes, pas une validation de la
  figure 3.
- **Le lien entre la valeur publiée 0,86 et les paramètres publiés** n'est pas établi : on ne sait pas si l'article a
  utilisé exactement ces valeurs, ni sur quelle fenêtre il a ajusté sa droite.
- **Le comportement sur d'autres plateformes** n'a pas été observé ici : la preuve locale a été jouée sous Linux ; la
  CI du dépôt rejoue Linux et Windows, mais ce document ne rapporte que les nombres de l'exécution locale.

## 11. Reproduire

```sh
cargo run --example figure_3
```

La sortie de cette commande est celle du § 6. Les tests d'acceptation de l'étape 3 s'exécutent avec la fonctionnalité
`etape-3` :

```sh
cargo test --features etape-3
```
