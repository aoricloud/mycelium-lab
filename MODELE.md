# Le modèle de l'étape 3 : la figure 3 de Meškauskas, Fricker et Moore (2004)

Ce document décrit le modèle tel que `src/voisinage.rs` l'implémente, et les nombres que
`examples/figure_3.rs` mesure. Il est écrit pour être lu sans connaître Lydite : les noms de champs du code sont
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

# Le chimiotropisme (étape 6)

Ce modèle appartient au couplage (`simulation::Simulation`, celui de la vitrine), pas au modèle de la figure 3 décrit plus
haut. Il ajoute une seule chose à une pointe : se pencher vers le côté où il y a plus de nutriments.

## Ce que fait le code

À chaque pas, juste après sa marche au hasard, une pointe lit le gradient du substrat dans la case où elle se trouve
(`Substrat::gradient` : des différences centrées ; un voisin hors de la grille ou bloqué est remplacé par la case
elle-même, comme dans la diffusion) et tourne de

`force · s · sin(angle du gradient − direction de la pointe)` degrés, avec `s = n / (n + G₀)`,

où `n` est la norme du gradient et `G₀ = 0,01` la constante de saturation : un gradient très faible tourne peu, un
gradient très fort tourne au plus de `force` degrés, à angle droit. La direction est un entier (un degré par indice de
table) : le virage réel est donc arrondi **au sort**, sa partie entière plus un degré avec la probabilité de sa partie
fractionnaire, de sorte qu'en moyenne le virage appliqué est exactement le virage réel. Le scénario règle la force par la
ligne `chimiotropisme <force>`, dans l'intervalle `[0 ; 45]`. À force nulle (le défaut), ou dans un gradient nul, la pointe
ne tire aucun nombre de plus : l'état est celui d'avant, octet pour octet, et `references/couplage-graine-1.empreinte` ne
change pas. Avec une force non nulle, l'écriture canonique de l'état se termine par la force (voir la documentation de
`Simulation::ecrire_canonique`).

## La référence, et ce qu'elle ne donne pas

Les références sont D. Turrà, M. El Ghalid, F. Rossi et A. Di Pietro, *Nature* 527:521–524 (2015) (le chimiotropisme de
*Fusarium oxysporum* vers des signaux de la plante hôte), et S. Hopkins et G. P. Boswell, *Fungal Ecology* 5:124–136
(2012) (un modèle de saut de vitesse de la réponse du mycélium à une répartition inégale des nutriments). **Nous n'avons
pu lire que les résumés de ces deux articles** : leurs méthodes, leurs figures et leurs équations ne sont pas lues, et
aucun test n'exige une valeur publiée. C'est donc une reproduction en esprit du chimiotropisme : la forme du virage
(`force · s · sin`), la constante `G₀ = 0,01`, la borne de 45° et l'arrondi tiré au sort sont des choix de modélisation,
ni calibrés ni tirés de ces articles.

## Ce que les tests vérifient

- À force nulle, le moteur de la V0 est inchangé bit à bit (les empreintes d'avant ne bougent pas).
- Pour une pointe seule, immobile, dans un gradient constant, l'orientation moyenne à l'équilibre est une valeur calculée
  exactement, hors du code testé (`tests/etape_6/chaine-t1-exact.py`, que le juge a recalculée par une autre méthode : mêmes
  huit chiffres). Pour une amplitude de marche de 20 et `p = force · s` : 0,2016 pour `p = 0,5`, 0,3814 pour 1, 0,6357 pour 2
  et 0,8410 pour 4.
- Plus la force est grande, plus la part de la colonie tournée vers la tache est grande (moyenne sur 100 graines, avec une
  marge mesurée).
- Les 63 tests ont été jugés avant d'être gelés : 113 fautes volontaires sur 120 sont tuées, les 7 autres n'ont aucun
  effet observable.

## Pourquoi la force est bornée à 45

L'orientation moyenne ne croît pas sans fin avec la force. À une amplitude de 20, elle vaut 0,9988 pour `p = 45`, 0,8905
pour `p = 120` et −0,216 pour `p = 360` : au-delà de `p` ≈ 57 (où `p · sin 1° = 1`), la pointe à un degré du gradient le
dépasse, et finit par le fuir. La borne de 45 garde « plus de force, plus d'orientation » vraie pour toutes les
amplitudes calculées (de 1 à 359).

## Ce qu'on voit

Sur le scénario `scenarios/chimiotropisme-sans.txt` (la spore en `(20, 60)`, une tache en `(42, 40)`, 100 pas), la part de
la biomasse qui finit à moins de 12 cases du centre de la tache, moyenne de 40 graines (`cargo run --release --example
mesure_chimiotropisme` refait ce tableau) :

| Force | Biomasse moyenne | Près de la tache | Colonies qui la touchent |
|---|---|---|---|
| 0 | 829,5 | 4,5 % | 19 sur 40 |
| 2 | 1 104,3 | 6,5 % | 23 sur 40 |
| 5 | 959,2 | 10,7 % | 24 sur 40 |
| 10 | 833,8 | 16,7 % | 24 sur 40 |
| 20 | 661,3 | 22,3 % | 25 sur 40 |
| 45 | 546,0 | 19,1 % | 26 sur 40 |

Les colonies de ce modèle varient beaucoup d'une graine à l'autre (certaines s'arrêtent très tôt) : une seule graine ne
prouve rien, la moyenne oui. Les aperçus `apercus/etape-6-chimiotropisme.png` et `apercus/etape-6-sans-chimiotropisme.png`
montrent la graine 7, choisie parmi les 40 parce que les deux colonies y ont à peu près la même taille et que la
différence s'y voit.

## Les limites

- Un virage borné à `force` degrés par pas, sans mémoire, fait tourner la pointe en spirale autour d'un maximum du champ au
  lieu de s'y arrêter : on le voit sur l'aperçu, où les hyphes s'enroulent autour de la tache. Nous ne prétendons pas que de
  vraies hyphes s'enroulent ainsi.
- Le gradient est lu dans la case de la pointe : une tache plus loin que la portée de la diffusion (de l'ordre de six
  cases pour la diffusion de ce scénario après 100 pas, d'après un calcul grossier) ne se sent presque pas.
- Le gradient lu est celui du champ entier, consommation de la colonie comprise : à force positive, une pointe se penche
  aussi vers les cases voisines que la colonie n'a pas encore épuisées, pas seulement vers les taches.
- Le tout est en 2D, sans obstacle autre que les cases bloquées du terrain ; le thigmotropisme est décrit dans la section suivante.
- Non vérifié : les méthodes, figures et valeurs des deux articles ; l'écriture canonique sur une machine gros-boutiste.

# Les obstacles et le thigmotropisme (étape 7)

Comme le chimiotropisme, ce modèle appartient au couplage (`simulation::Simulation`, celui de la vitrine), pas au modèle de
la figure 3. Il ajoute deux choses : **un obstacle n'est jamais touché**, et ce qu'une pointe fait quand son pas la mènerait
dedans (le thigmotropisme : *thigmo* vient du toucher).

## Ce que fait le code

**La règle du plancher.** Chaque point `(x, y)` du plan est dans la case `(⌊x⌋, ⌊y⌋)`, et un segment est *libre* si aucun de
ses points, extrémités comprises, n'est dans une case bloquée (`Substrat::segment_libre`). Cette règle vaut toujours, même
sans thigmotropisme. Si le segment qu'une pointe poserait n'est pas libre, c'est un **contact**.

**À `thigmotropisme 0` (le défaut)**, une pointe qui touche reste où elle est, avec sa nouvelle direction (comme le cas
« apex bloqué » de la V0), sans laisser de segment ni consommer, et retente au pas suivant.

**À `thigmotropisme a > 0`** (`a` en degrés, dans `[0 ; 90]`), on regarde la face touchée (`Substrat::face_de_contact` : la
ligne de la grille franchie pour entrer dans la première case bloquée) et l'angle de contact `α` entre la direction de la
pointe et cette face (0 pour le parallèle, 90 pour la perpendiculaire ; des entiers, aucune trigonométrie) :

- `α < a` : la pointe **longe** l'obstacle. Elle prend celle des deux directions parallèles à la face qui dévie le moins de la
  sienne (`g`), garde en mémoire son cap d'avant (`cap`) et `g`, et refait son pas dans la direction `g`. À chaque pas
  suivant, elle tire un écart `δ` entre -20 et 20 degrés ; si un pas plein dans la direction `cap` est libre (le bout du mur
  est passé) et si son pas réel dans la direction `cap + δ` l'est aussi, elle **retrouve son cap** (`cap + δ`) et la mémoire
  s'efface ; sinon elle continue de longer. Une pointe qui longe n'est pas soumise au chimiotropisme.
- `α ≥ a` : la pointe **s'arrête et se ramifie**. Elle devient inactive et, dans chacune des deux directions parallèles à la
  face (la plus petite d'abord) où un pas plein est libre, une pointe neuve naît à sa position, **sans mémoire**. Chaque
  naissance compte comme une ramification. Dans un coin concave, la direction tournée vers le mur voisin n'a pas la place :
  la pointe n'y naît pas.
- Au bord de la boîte, une pointe qui ne peut plus avancer reste où elle est et perd sa mémoire.

Une anastomose ne se fait pas à travers un obstacle : un candidat que sépare un obstacle est écarté et la recherche continue.
Le scénario règle l'angle par la ligne `thigmotropisme <angle>`. Avec obstacles et `thigmotropisme 0`, l'état change par
rapport à T0 dès qu'une pointe touche un mur (les obstacles sont durs) ; sans contact, ou sans obstacle, il est celui
d'avant, octet pour octet. Avec un angle non nul, l'écriture canonique de l'état se termine par l'angle puis, pour chaque
pointe, la mémoire qu'elle porte (voir `Simulation::ecrire_canonique`).

## La référence, et ce qu'elle ne donne pas

La référence est M. Held, O. Kašpar, C. Edwards et D. V. Nicolau, « Intracellular mechanisms of fungal space searching in
microenvironments », *PNAS* 116(27):13543–13552 (2019), *Neurospora crassa* dans des structures microfluidiques. **Elle a été
lue en entier (texte complet) par le juge des tests**, pas seulement le résumé. Ce que l'article donne, et ce que le code en
reprend :

| Ce que dit l'article | Dans le code |
|---|---|
| Une hyphe qui touche un obstacle sous moins de 35° de sa surface la longe (« nestling », 26 hyphes imagées), « dans la direction de moindre déviation ». | Le seuil `thigmotropisme 35` ; la direction de glissement `g` est la parallèle la plus proche. |
| Au-delà, sa pointe se divise (« hit & split », 37 événements imagés), en T, les deux branches partant en sens opposés le long de l'obstacle ; **la mémoire de direction est perdue**. | L'arrêt, et deux pointes nées sans mémoire dans les deux sens parallèles à la face. |
| Après le bout de l'obstacle, l'hyphe retrouve son cap sur une distance d'environ un diamètre (7 µm), à environ 20° près. | Le retour au cap au plus un pas après le bout du mur, à ±20° ; le « 20° » est cité par l'article d'après les travaux antérieurs du groupe, il n'y est pas mesuré. |
| Quand la place manque (un coin, un obstacle court), une seule branche se forme. | Une pointe ne naît que si un pas plein dans sa direction est libre. |

**Ce que l'article ne donne pas, et que nous avons choisi** : un seuil net à 35° (les 37 collisions imagées sont presque
perpendiculaires : entre 35° et près de 80°, rien n'est montré) ; un glissement exactement selon un axe de la grille, à
la distance du contact ; la loi uniforme de l'écart `δ` ; un arrêt instantané (dans l'article : environ deux minutes) ; le
retour décidé par une sonde. Le 35° est un paramètre repris de la mesure, pas un résultat du modèle. Les « environ 45° » de
l'article sont l'angle des ramifications latérales en milieu ouvert, non du hit & split : ils ne sont pas dans le code.
Les travaux antérieurs que l'article cite (canaux étroits : extension jusqu'à dix fois plus lente, ramifications cinq à
dix fois plus rapprochées) n'ont pas été lus.

## Ce que les tests vérifient

- La règle du plancher et la face d'un contact, **exactement** : 80 000 segments comparés à un oracle écrit dans les tests
  (des fractions d'entiers, sans arrondi), coins et lignes de la grille compris.
- Les trois fonctions pures (`angle_de_contact`, `direction_de_glissement`, `branches_de_contact`) sur les 360 directions et
  les deux faces ; le seuil (`α < a` longe, `α ≥ a` s'arrête) pour sept angles ; le retour au cap (au plus 20°, au plus un
  pas après le bout du mur) ; le bord de la boîte ; le coin concave ; les pas autres que 1.
- **L'état complet, octet pour octet**, de 324 colonies à marche au hasard, suivies pas à pas par un modèle de référence
  écrit à part dans les tests.
- Jamais un segment dans un obstacle, sur 312 colonies, huit dispositifs, quatre angles et trois pas.
- L'anastomose qui ne traverse pas un mur (un dispositif déterministe, et son témoin sans mur) ; les refus d'angle ;
  l'écriture canonique ; la lecture du scénario ; le binaire ; et, sans obstacle ou sans contact, **l'état d'avant, à
  l'octet**.
- En moyenne sur 120 graines du scénario `scenarios/mur.txt`, avec une marge d'au moins cinq écarts-types sur d'autres jeux
  de graines : à 35° la colonie épouse le mur (au moins 3 points de biomasse collée de plus qu'à angle nul) et s'y ramifie
  (au moins 1,3 fois plus de ramifications) ; à 90° elle le contourne (au moins 2 points de biomasse de plus au-delà).
- Les 66 tests ont été jugés avant d'être gelés : 112 fautes volontaires sur 117 sont tuées, les 5 autres n'ont aucun effet
  observable.

## Ce qu'on voit

Sur `scenarios/mur.txt` (une spore en `(20, 50)`, un mur de 4 colonnes sur 61 lignes devant elle, 120 pas), moyenne de 40
graines (`cargo run --release --example mesure_thigmotropisme` refait ce tableau) :

| Angle critique | Biomasse moyenne | Au-delà du mur | Part au-delà | Part collée au mur | Ramifications |
|---|---|---|---|---|---|
| 0 | 738,8 | 140,7 | 19,0 % | 3,5 % | 39,6 |
| 15 | 1 797,8 | 359,9 | 20,0 % | 11,8 % | 138,8 |
| 35 | 1 327,4 | 326,3 | 24,6 % | 11,1 % | 81,3 |
| 60 | 1 410,1 | 416,7 | 29,5 % | 10,6 % | 80,8 |
| 90 | 1 271,5 | 443,3 | 34,9 % | 9,9 % | 69,7 |

À 35°, la colonie **épouse le mur et s'y ramifie** : elle est environ deux fois plus grosse qu'à angle nul, et en passe environ
deux fois plus de biomasse de l'autre côté. Mais la **part** de sa biomasse au-delà du mur n'augmente presque pas (24,6 %
contre 19,0 % sur ces 40 graines ; de 3 points en moyenne sur d'autres jeux de graines) : « elle le contourne » n'est vrai
qu'aux grands angles, où presque tout est longé. À angle nul, une pointe qui touche le mur y reste collée sans laisser
de segment, jusqu'à ce que sa marche au hasard l'en détourne. Les colonies varient beaucoup d'une graine à l'autre (certaines
ne touchent jamais le mur) : une seule graine ne prouve rien, la moyenne oui. Les aperçus `apercus/etape-7-mur.png` et
`apercus/etape-7-sans-thigmotropisme.png` montrent la graine 31 : à 35°, les hyphes s'accumulent le long du mur et s'y
ramifient ; à angle nul, quelques-unes seulement l'effleurent.

## Les limites

- Un seul angle critique, net, pour une grille aux murs droits ; l'article n'image le comportement entre 35° et la
  perpendiculaire que près de la perpendiculaire.
- Le glissement est axial, à la distance du premier contact (moins d'un pas) : la pointe n'est pas avancée jusqu'à la face.
- L'arrêt est instantané, la ramification d'un coin ou d'un obstacle court ne se fait que si la place d'un pas existe ; pas
  de ralentissement en canal étroit, pas de ramification vers une ouverture latérale : non modélisés.
- La mémoire est un cap retrouvé à ±20° (loi uniforme choisie), au plus un pas après le bout du mur ; elle s'efface à chaque
  retour et au bord de la boîte. L'article dit que la précision du retour ne se dégrade pas sur des virages successifs : ici
  les écarts `δ` s'ajoutent d'un contact à l'autre.
- L'échelle (une case, un pas) n'est pas calibrée sur le diamètre d'une hyphe (environ 7 µm).
- Le passage exact par un coin suit la règle du plancher, sans signification physique (la mesure en est nulle).
- Non testé directement : qu'un candidat d'anastomose écarté laisse chercher un autre plus loin (le dispositif de test ne le
  distingue pas de « pas d'anastomose à ce pas »), le comportement à un ULP près d'une ligne de la grille quand les
  coordonnées ne sont pas des flottants exacts, et le contact avec un substrat qui diffuse ou s'épuise comparé à l'octet à un
  modèle (couvert par des propriétés générales et des moyennes).
- Non vérifié : les travaux antérieurs que cite l'article (canaux étroits, mémoire à 20°) ; l'écriture canonique sur une
  machine gros-boutiste.

# La pièce que la colonie remplit (étape 9, M0)

Cette étape **mesure** : elle ne change rien à la simulation (`piece::mesurer` prend `&Simulation`), et les empreintes de la
V0 et de T0 à T4 restent celles du dépôt, à l'octet. Elle ne parle ni de raideur ni de résistance : le banc ne prédit pas la
résistance d'un vrai matériau. Ce sont des définitions et des invariants exacts, sans article de référence ; aucun chiffre de
l'étude du matériau marqué « à vérifier » n'y est employé.

## Ce que fait le code

**La carte de densité** (`piece::carte_de_densite`). Pour chaque case, la longueur d'hyphe qu'elle contient. Chaque segment de
la colonie est coupé aux lignes de la grille qu'il franchit ; chaque morceau appartient à la case qui contient son milieu
(règle du plancher, comme T2) et lui apporte sa longueur. Un segment qui longe le bord haut ou droit de la boîte compte dans la
dernière ligne ou colonne. Un passage exact par un coin laisse zéro aux cases qui ne font que toucher le coin. La somme des
cases est la somme des longueurs des segments, c'est-à-dire la biomasse, à l'arrondi près.

**Un seuil de densité.** Une case est *occupée* à un seuil `s ≥ 0` si sa densité est strictement supérieure à `s`. Au seuil
nul, toute case où passe de l'hyphe est occupée ; or une colonie part d'une seule spore et ses segments se suivent : ses cases
occupées sont d'un seul tenant. Le seuil donne un sens aux mesures qui suivent : à une longueur d'hyphe par case (seuil 1), les
colonies de la vitrine se découpent en plusieurs morceaux.

**Les morceaux et la traversée.** Deux cases occupées sont voisines si elles se touchent par un côté ou un coin (huit
voisins) : un hyphe qui passe exactement par un coin de case ne laisse que des cases en diagonale, qui font un seul morceau.
`piece::morceaux` rend la taille de chaque morceau ; `piece::traversee` le nombre de cases du plus court chemin de cases
occupées d'un bord de la boîte à l'autre, ou rien.

**Le bilan de nourriture.** Ce que la colonie a consommé est `cout × biomasse` ; la nourriture restante plus la nourriture
consommée égale la nourriture de départ, à l'arrondi près : l'écart est d'au plus 7,1·10⁻¹¹ sur une nourriture de 9 756 à
25 921, à chaque pas des six scénarios de la vitrine (un test l'exige à 10⁻⁹ près en relatif). Le substrat diffuse sans perte
et l'hyphe consomme exactement `cout` par unité de longueur : c'est une propriété du modèle que M0 rend visible.

**`piece::mesurer` et `piece::mesurer_au_seuil`** rendent une `MesuresPiece` : cases libres et occupées, remplissage (cases
occupées sur cases libres), densité moyenne et maximale, biomasse, les quatre nombres du bilan, le plus grand morceau, le
nombre de morceaux et les deux traversées (gauche à droite, haut à bas). `mesurer` est le seuil nul. L'exemple
`cargo run --release --example mesure_piece -- <scénario> <graine> [<dossier>]` imprime ces quatorze nombres, un par ligne, et
écrit `densite.png`, la carte de couleurs de la densité (du violet au jaune quand elle monte ; le mur en gris, le fond sombre).
Le binaire `mycelium` et la page ne changent pas.

## Ce que les tests vérifient

- **La carte, exactement** : des cas à la main (un segment dans une case, sur plusieurs, en diagonale par des coins, le long
  d'une ligne de grille, fini sur une ligne ou sur le bord), comparée à des fractions exactes pour des extrémités en huitièmes
  de case et à la méthode des dalles pour des extrémités quelconques ; l'ordre des additions, au bit près ; la nullité exacte
  des cases que le segment ne traverse pas.
- **Sur la simulation** : la somme de la carte égale la biomasse ; la densité est nulle dans les cases bloquées ; le bilan de
  nourriture tient à chaque pas ; `mesurer` ne change pas l'état ; les six empreintes de référence ne bougent pas.
- **Les morceaux et la traversée** : des grilles écrites à la main et des grilles au hasard, comparées à deux calculs
  indépendants (une union de classes, des relaxations successives).
- **Le seuil** : strictement supérieur, la monotonie (quand il monte, les cases occupées se raréfient, le plus grand morceau ne
  grandit pas, une traversée ne naît pas).
- **L'exemple** : sa sortie caractère pour caractère, chaque pixel de l'image, le code de sortie 2 en cas d'erreur.
- 82 tests ; un juge a joué 209 fautes volontaires, 188 sont tuées ; les 21 autres sont sans effet observable, chacune classée
  avec sa raison (trois ne le sont que sur les colonies d'aujourd'hui).

## Ce qu'on voit

Mesures au seuil 0 puis au seuil 1 (une longueur d'hyphe par case), à la fin de la durée de chaque scénario de la vitrine
(`cargo run --release --example mesure_piece` refait le seuil 0 ; le seuil 1 passe par `mesurer_au_seuil`) :

| Scénario (graine, pas) | Cases occupées, seuil 0 | Remplissage | Cases occupées, seuil 1 | Morceaux, seuil 1 | Plus grand morceau, seuil 1 |
|---|---|---|---|---|---|
| libre (1, 80) | 1 129 | 4,4 % | 638 | 80 | 119 |
| terrain (1, 80) | 113 | 1,2 % | 74 | 6 | 51 |
| tache (7, 100) | 753 | 7,5 % | 438 | 39 | 237 |
| tache-sans (7, 100) | 915 | 9,1 % | 535 | 57 | 207 |
| mur (31, 120) | 2 085 | 21,4 % | 1 323 | 98 | 853 |
| mur-sans (31, 120) | 1 177 | 12,1 % | 726 | 59 | 424 |

Au seuil nul, chaque colonie est d'un seul morceau. Sur ces six colonies, seules celles du mur (avec et sans thigmotropisme)
touchent à la fois le haut et le bas de la boîte (la traversée haut-bas vaut 100 cases) ; aucune ne va du bord gauche au bord
droit, et au seuil 1 aucune traversée ne reste. Sur la graine 31, la colonie du mur à 35° est environ deux fois plus dense que
sans thigmotropisme (densité moyenne 0,261 contre 0,141) et plus remplie (21,4 % contre 12,1 %) : c'est l'accumulation le long
du mur que T2 décrit, vue par la carte ; une seule graine ne prouve rien, la moyenne sur 40 graines est dans la partie sur T2.
L'aperçu `apercus/etape-9-densite.png` montre cette carte pour `scenarios/mur.txt`, graine 31.

## Les limites

- La densité est une **longueur d'hyphe par case**, pas un diamètre, une masse volumique ou une porosité mesurée : une case
  traversée par un long hyphe et une case qui contient plusieurs brins courts de même longueur totale ne se distinguent pas.
- Le **seuil** est un choix de M0 (strictement supérieur), sans valeur physique ; les morceaux dépendent de lui.
- Les **huit voisins** sont un choix de modélisation (voir plus haut).
- La **traversée** compte des cases occupées, pas des hyphes continus : deux hyphes voisines qui ne se sont pas anastomosées
  font un chemin de cases occupées ; le calcul de raideur (M3) devra regarder les liaisons réelles.
- Le tout est en 2D, à l'échelle de l'hyphe ; rien n'est calé sur un vrai moule.
- L'image est à 4 pixels par case (2 pour une boîte de 161 × 161, pour rester sous 500 Ko) ; au-delà d'environ 404 × 404 cases,
  même à 1 pixel par case elle dépasse 500 Ko (aucun scénario n'y arrive ; ni testé, ni corrigé).
- Rien n'a été vérifié sous Linux par le juge (le dépôt le fait par sa CI).

# Le moule (étape 10, M1)

Cette étape donne une **forme** à l'espace libre de la boîte et mesure comment la colonie la remplit. Elle ne change rien au
noyau : les empreintes de la V0 et de T0 à T4 restent celles du dépôt, à l'octet, et les invariants de M0 (somme de la
carte = biomasse, densité nulle dans les cases bloquées, bilan de nourriture) tiennent dans tout moule. Elle ne parle ni de
raideur ni de résistance : le banc ne prédit pas la résistance d'un vrai matériau. Un moule est un modèle à nous, sans
article de référence : aucune publication ne valide « la colonie remplit un moule de cette forme ». T3 (le labyrinthe, le
temps de sortie, A* et la profondeur d'abord) est repliée ici : le labyrinthe est une forme de moule.

## Ce que fait le code

**Les formes** (`moule::Forme`, `moule::libres`). Une brique (un rectangle de cases), un L (la barre de gauche et la barre
du bas d'un rectangle, d'une épaisseur donnée), un disque et un anneau (les cases dont le carré de la distance entière au
centre est au plus `rayon²`, et pour l'anneau strictement plus que `rayon_interieur²`), et un labyrinthe. Tout se calcule en
entiers ; une forme hors contrat est refusée (`FormeInvalide`), une cavité qui sort de la boîte aussi (`HorsDeLaBoite`), sans
jamais paniquer, même pour des paramètres énormes.

**La décomposition** (`moule::obstacles`). Le complément de la cavité est découpé en rectangles disjoints, d'une façon fixée
pour que tous écrivent les mêmes lignes : ligne par ligne, les suites de cases bloquées, chacune prolongeant le rectangle de
mêmes bornes de la ligne du dessus s'il existe ; le résultat est trié par `(y0, x0)`.

**Le texte du scénario** (`moule::developper`). Une ligne `moule brique x0 y0 x1 y1`, `moule l …`, `moule disque …`,
`moule anneau …` ou `moule labyrinthe colonnes rangees couloir graine` est remplacée, à sa place, par les lignes
`obstacle x0 y0 x1 y1` de la décomposition ; tout le reste du texte est conservé octet pour octet. Le noyau, le binaire et la
page lisent le texte développé : un scénario de moule est pour eux un scénario ordinaire. La ligne `boite` doit venir avant ;
les erreurs sont lues dans un ordre fixe (syntaxe, doublon, boîte absente, puis la forme).

**Le labyrinthe.** `colonnes × rangees` chambres carrées de `couloir` cases, séparées par des murs d'une case, dans un
contour ; un passage ouvre les `couloir` cases du mur commun. La génération est une recherche en profondeur aléatoire
(voisines dans l'ordre haut, droite, bas, gauche, un tirage de `Alea` à chaque choix, même quand il n'y a qu'une voisine) :
un labyrinthe **parfait**, `colonnes × rangees − 1` passages, un seul chemin entre deux chambres au niveau des chambres,
entièrement déterminé par sa graine. La spore est au centre de la chambre `(0, 0)` ; la sortie est la chambre
`(colonnes − 1, rangees − 1)` entière (`moule::chambre`).

**Les chemins.** Trois recherches sur la grille des cases libres, **à quatre voisins** (par les côtés) : le plus court
chemin par un parcours en largeur, A* avec la distance de Manhattan (admissible et cohérente : même réponse), et la
profondeur d'abord itérative (une pile explicite), qui rend le premier chemin simple qu'elle trouve. La profondeur égale le
plus court chemin seulement quand un seul chemin simple existe : dans un labyrinthe de couloir 1, dont les cases libres
forment un arbre ; avec un couloir plus large, elle tourne dans les chambres et rallonge (92 pas contre 52 sur le scénario).

**La courbe de remplissage** (`moule::courbe_de_remplissage`, `moule::pas_pour_remplir`) : à chaque pas, la part des cases
de la cavité où passe de l'hyphe (`piece::mesurer`, seuil nul) ; elle est croissante au sens large, une case occupée le
reste. `pas_pour_remplir` donne le premier pas où une part est atteinte.

**Le temps de sortie** (`moule::temps_de_sortie`) : le premier pas, au plus `pas_max`, où un segment finit dans un
**rectangle de cases** cible (la case de la spore dans le rectangle vaut 0). La simulation n'avance que ce qu'il faut, et le
résultat ne dépend pas du nombre de pas déjà joués. La cible est un rectangle, pas une case : avec la seule case centre de
la dernière chambre, une colonie qui remplissait 91 % du labyrinthe pouvait être dite « pas sortie » parce qu'aucun segment
ne finissait exactement dans cette case.

**L'exemple** `cargo run --release --example mesure_moule -- <scénario> <graine> [<dossier>]` développe le scénario, joue
sa durée, imprime la taille de la cavité, la courbe tous les 10 pas, les quatorze mesures de M0 et, pour un labyrinthe, le
temps de sortie et le plus court chemin ; avec un dossier, il écrit `densite.png` (le dessin de `mesure_piece`).

## Ce que les tests vérifient

- **Les formes, exactement** : des dessins à la main, les aires par formule (le disque par les comptages de Gauss), les
  contrats à chaque borne, la symétrie, des formes au hasard contre un prédicat écrit à part.
- **La décomposition** : des cas à la main, les propriétés (disjoints, dans la boîte, union exacte, triés), des formes au
  hasard contre une décomposition de référence écrite autrement.
- **Le texte** : le texte rendu au caractère, l'identité octet pour octet sans ligne `moule`, chaque erreur et leur ordre,
  les lignes `boite` mal formées, les blancs, les cinq scénarios contre une référence.
- **Le labyrinthe** : deux labyrinthes dérivés à la main des sorties du générateur, une génération récursive de référence sur
  des centaines de tailles, couloirs et graines, les propriétés d'un labyrinthe parfait, les plus petits labyrinthes.
- **Les chemins** : des grilles à la main, A* contre des relaxations successives sur des milliers de grilles au hasard, la
  profondeur contre une récursion de référence, pas de débordement de pile sur 200 × 200.
- **Dans la simulation** : les invariants de M0 dans chaque moule, la courbe croissante et égale à `mesurer` à chaque pas,
  le temps de sortie égal à un recalcul indépendant, indépendant du nombre de pas déjà joués, minoré par la distance au
  rectangle, chaque bord du rectangle compté ; les six empreintes de référence inchangées ; la sortie de l'exemple au
  caractère et son image au pixel.
- Ils **ne vérifient pas** : que les réglages des scénarios sont les bons (ils figent des planchers de remplissage sous
  le plus mauvais de 30 graines, pas une valeur) ; la biologie d'une pointe contre une paroi ; rien sous Linux (la CI du
  dépôt le fait).
- 106 tests ; un juge a joué 212 fautes volontaires dans `moule.rs` et l'exemple, 201 sont tuées ; les 11 autres sont
  sans effet observable, chacune classée avec sa raison.

## Ce qu'on voit

Les réglages des cinq scénarios sont ceux des scénarios existants (`substrat 1.0 0.2`, `couplage 0.1 0.002`,
`anastomose 0.5 10`) avec deux changements, mesurés sur 30 graines : l'**intervalle de ramification vaut 8** au lieu de 20
(`croissance 1.0 8.0 30 20`) et le **thigmotropisme 35** de `mur.txt` est ajouté. Dans une cavité fermée, chaque pointe finit
par s'anastomoser ou par s'arrêter contre la paroi : la colonie s'éteint toujours (plus aucune pointe), et la question est
ce qu'elle a rempli avant. Avec les réglages existants à la lettre, elle s'éteint avec 20 à 45 % de la cavité ; sans
thigmotropisme, une pointe contre la paroi reste immobile et la colonie ne sort jamais du labyrinthe. À l'intervalle 8, elle
remplit en moyenne 87 % de la brique, 93 % du L, 87 % du disque, 96 % de l'anneau et 99 % du labyrinthe, et elle sort du
labyrinthe pour chacune des 30 graines.

Remplissage final (part des cases libres où passe de l'hyphe, seuil nul, à la fin de la durée), pas pour atteindre 50 % et 90 % (`pas_pour_remplir`
sur la courbe pas à pas ; `—` : jamais atteint dans la durée), et pour le labyrinthe le temps de sortie (`temps_de_sortie` vers la chambre (5, 5), `pas_max` =
la durée) et le plus court chemin (`plus_court_chemin`, 4 voisins, de centre à centre). Graines de simulation 1 à 10 ; l'exemple refait chaque ligne
(`cargo run --release --example mesure_moule -- scenarios/moules/<nom>.txt <graine>`) : `remplissage`, les lignes `courbe`, `sortie` et `chemin`.

| Scénario | Graine | Remplissage final | Pas pour 50 % | Pas pour 90 % | Temps de sortie | Plus court chemin |
|---|---|---|---|---|---|---|
| Brique (1 200 cases, durée 250) | 1 | 92,5 % | 48 | 66 | — | — |
|  | 2 | 85,9 % | 49 | — | — | — |
|  | 3 | 84,8 % | 77 | — | — | — |
|  | 4 | 67,6 % | 66 | — | — | — |
|  | 5 | 92,9 % | 62 | 76 | — | — |
|  | 6 | 81,9 % | 57 | — | — | — |
|  | 7 | 86,2 % | 51 | — | — | — |
|  | 8 | 96,1 % | 47 | 59 | — | — |
|  | 9 | 88,4 % | 59 | — | — | — |
|  | 10 | 90,8 % | 56 | 75 | — | — |
| L (1 575 cases, durée 250) | 1 | 97,2 % | 50 | 74 | — | — |
|  | 2 | 86,9 % | 60 | — | — | — |
|  | 3 | 88,1 % | 133 | — | — | — |
|  | 4 | 92,3 % | 68 | 91 | — | — |
|  | 5 | 93,7 % | 56 | 78 | — | — |
|  | 6 | 97,0 % | 58 | 91 | — | — |
|  | 7 | 86,9 % | 49 | — | — | — |
|  | 8 | 97,5 % | 52 | 79 | — | — |
|  | 9 | 94,6 % | 54 | 78 | — | — |
|  | 10 | 91,7 % | 59 | 94 | — | — |
| Disque (1 961 cases, durée 250) | 1 | 87,9 % | 52 | — | — | — |
|  | 2 | 75,4 % | 65 | — | — | — |
|  | 3 | 82,2 % | 87 | — | — | — |
|  | 4 | 94,4 % | 70 | 88 | — | — |
|  | 5 | 87,8 % | 66 | — | — | — |
|  | 6 | 87,1 % | 56 | — | — | — |
|  | 7 | 90,2 % | 63 | 85 | — | — |
|  | 8 | 94,4 % | 61 | 80 | — | — |
|  | 9 | 76,2 % | 76 | — | — | — |
|  | 10 | 91,6 % | 67 | 89 | — | — |
| Anneau (2 336 cases, durée 250) | 1 | 93,8 % | 77 | 115 | — | — |
|  | 2 | 95,0 % | 70 | 108 | — | — |
|  | 3 | 90,3 % | 85 | 122 | — | — |
|  | 4 | 95,1 % | 71 | 108 | — | — |
|  | 5 | 94,4 % | 81 | 118 | — | — |
|  | 6 | 95,2 % | 77 | 113 | — | — |
|  | 7 | 98,8 % | 69 | 104 | — | — |
|  | 8 | 98,5 % | 68 | 106 | — | — |
|  | 9 | 97,0 % | 98 | 133 | — | — |
|  | 10 | 97,1 % | 69 | 105 | — | — |
| Labyrinthe (429 cases, durée 300) | 1 | 99,3 % | 84 | 134 | 69 | 52 |
|  | 2 | 99,8 % | 79 | 134 | 63 | 52 |
|  | 3 | 100,0 % | 80 | 131 | 65 | 52 |
|  | 4 | 98,1 % | 84 | 135 | 66 | 52 |
|  | 5 | 98,1 % | 82 | 133 | 65 | 52 |
|  | 6 | 100,0 % | 78 | 130 | 63 | 52 |
|  | 7 | 100,0 % | 76 | 128 | 61 | 52 |
|  | 8 | 100,0 % | 77 | 128 | 61 | 52 |
|  | 9 | 97,9 % | 88 | 141 | 70 | 52 |
|  | 10 | 99,8 % | 86 | 136 | 71 | 52 |

Sur les dix graines :

- Brique : moyenne 86,7 %, de 67,6 à 96,1 % ; 50 % atteint 10 fois sur 10, 90 % 4 fois sur 10.
- L : moyenne 92,6 %, de 86,9 à 97,5 % ; 50 % atteint 10 fois sur 10, 90 % 7 fois sur 10.
- Disque : moyenne 86,7 %, de 75,4 à 94,4 % ; 50 % atteint 10 fois sur 10, 90 % 4 fois sur 10.
- Anneau : moyenne 95,5 %, de 90,3 à 98,8 % ; 50 % atteint 10 fois sur 10, 90 % 10 fois sur 10.
- Labyrinthe : moyenne 99,3 %, de 97,9 à 100,0 % ; 50 % atteint 10 fois sur 10, 90 % 10 fois sur 10.

Le labyrinthe du scénario (6 × 6 chambres de couloir 3, graine 5, 429 cases libres) a un plus court chemin de **52 pas**
du centre de la chambre (0, 0) au centre de la chambre (5, 5) (A* : 52 ; profondeur d'abord : 92). Une pointe avance d'au
plus 0,909 par pas (`pas × s / (affinite + s)` avec `s = 1`) : 57 pas au moins ; la colonie sort entre 61 et 71 pas pour les
graines 1 à 10, et de 61 à 71 sur 30 graines. Les aperçus `apercus/etape-10-labyrinthe.png` (graine 1) et
`apercus/etape-10-disque.png` (graine 2) montrent la carte de densité.

## Les limites

- **Un modèle à nous.** Aucune publication ne décrit une colonie qui remplit un moule de ces formes, ni ne donne un temps
  de sortie de labyrinthe à comparer ; les nombres ci-dessus sont ceux du modèle, pas d'une expérience.
- **La règle de T2 contre une paroi de moule.** L'angle critique de 35° (Held et al. 2019) a été mesuré dans des canaux de
  quelques micromètres ; rien ne dit qu'il vaut contre la paroi d'un moule, et il décide pourtant du résultat de chaque
  scénario (sans lui, la colonie reste collée aux parois).
- **Les réglages sont choisis pour montrer quelque chose**, pas calibrés : l'intervalle 8 et le thigmotropisme 35 font
  remplir la cavité ; aux réglages de la vitrine, la colonie s'éteint à moitié chemin.
- **Ce qui reste mauvais.** La colonie n'entre jamais dans une **poche** qu'elle a refermée : une pointe qui touche une
  hyphe s'anastomose et meurt, donc une région entourée d'hyphes sans pointe dedans reste vide pour toujours (les taches
  sombres des aperçus) ; c'est la dynamique du noyau (anastomose, intervalle de ramification), pas un défaut des formes :
  on l'observe aussi en boîte ouverte. Les hyphes **s'accumulent le long des parois** (T2 : une pointe qui longe la paroi y
  reste, et celles qui la frappent de face s'y ramifient) : la densité est la plus forte au bord de la cavité. Le plus
  mauvais remplissage sur 30 graines est de 68 % (brique), 84 % (L), 69 % (disque), 88 % (anneau), 96 % (labyrinthe).
- **Une densité sans limite.** Le noyau ne borne pas la longueur d'hyphe par case : des milliers de pointes vivent un temps dans
  des cases déjà pleines, et la densité moyenne atteint des dizaines à des centaines de longueurs par case (labyrinthe, graine 1 :
  204, pour une biomasse de 87 500 sur 429 cases). Rien de physique là-dedans ; M2 devra mettre une règle de ramification selon la
  place.
- **Quatre voisins, des cases carrées, le 2D.** Les chemins vont de case en case par les côtés : ce n'est pas le plus court
  chemin géométrique ; les formes courbes sont des escaliers de cases ; rien n'est calé sur un vrai moule, une case n'a pas
  d'échelle physique (M2).
- **Les traversées de M0** (`traversee_gauche_droite`, `traversee_haut_bas`) valent toujours `aucune` sur ces cinq
  moules : elles vont d'un bord de la **boîte** à l'autre et aucune de ces cavités ne touche deux bords opposés ; c'est
  normal, les deux dernières lignes de l'exemple n'apportent rien ici.
- Le temps de sortie compte la **fin d'un segment** dans le rectangle : une hyphe qui traverse la chambre sans y finir un
  pas n'est pas comptée ; avec une chambre de 9 cases et un pas de 0,909, le cas ne s'est pas présenté sur 30 graines.
- Rien n'a été vérifié sous Linux par le juge (le dépôt le fait par sa CI).

# La carte de densité sur la page (étape 11)

Cette étape **montre** : elle ne change rien à la simulation ni au calcul de la carte. La vitrine apprend à dessiner la carte de
densité de M0 (`piece::carte_de_densite`) à la place du substrat, quand on coche la case « Densité ». Le noyau, le binaire
`mycelium` et les empreintes de la V0, de T0 à T4, de M0 et de M1 restent ceux du dépôt, à l'octet.

## Ce que fait le code

**Deux exportations de plus** dans `src/vitrine.rs` (18 en tout). `mycelium_densite(x, y)` rend la longueur d'hyphe de la case
`(x, y)` de la simulation en cours, la même au bit près que `piece::carte_de_densite` ; `mycelium_densite_max()` rend la plus
grande case, la même que `densite_maximale` de `piece::mesurer`. Hors de la boîte, et avant tout démarrage, elles rendent
`NaN` ; une carte vide (le temps 0) vaut 0.0 partout et son maximum vaut 0.0.

**La carte en cache.** La carte est gardée dans l'état de la vitrine. Un démarrage ou un pas de temps la rend périmée ; elle se
recalcule à la première lecture qui suit, puis sert à toutes les lectures jusqu'au changement suivant : la page lit jusqu'à
161 × 161 cases par image sans la recalculer, et une page qui ne coche jamais la case ne la calcule jamais. `avancer(0)` et le
choix d'un scénario ne la touchent pas.

**La rampe.** Cochée, la page peint une case par pixel d'une couche qu'elle agrandit sans lissage (des carrés nets, comme
`densite.png`). Une case sans hyphe a la couleur du fond (16, 16, 24) ; une case où passe de l'hyphe a le niveau
`n = ⌊√min(d / dmax, 1) × 255⌋` puis, canal par canal, `violet + (jaune − violet) × n / 255` en entiers arrondis vers zéro, du
violet (70, 20, 110) au jaune (255, 240, 90) : la rampe de `couleur_de_densite` dans `examples/mesure_piece.rs`. La racine
étire les faibles densités : une colonie paraît surtout rose-violet, avec du jaune aux carrefours et le long des murs. Les
obstacles restent dessinés comme sur la page (ardoise et liseré clair, pas le gris de `densite.png`) ; il n'y a ni substrat,
ni vignettage, ni tracé des hyphes ; le cadre ne change pas.

**La légende.** « Densité d'hyphe : peu [une barre du violet au jaune] beaucoup » se montre avec la carte ; les légendes du
substrat et de la tache se cachent, puisque le substrat n'est plus dessiné, et reviennent à la décoche. La barre est un dégradé
linéaire entre les deux bouts de la rampe : linéaire en niveau, pas en densité.

## Ce que les tests vérifient

- **La carte du module est celle de M0**, case par case et bit à bit, en natif (six scénarios, cinq graines, cinq moments, une
  colonie suivie jusqu'à la fin de sa croissance) et sous Node avec le vrai module (six scénarios, trois graines, des pas
  hachés), avec son maximum ; la somme des cases est la biomasse ; les cases des murs valent zéro ; `NaN` hors de la boîte, y
  compris pour une ligne qui, en WebAssembly 32 bits, reviendrait dans la carte si elle n'était pas contrôlée.
- **Le cache** : la carte change après un pas, pas après `avancer(0)` ; un démarrage la vide ; l'ordre des lectures n'y fait rien.
- **Le module** : exactement 18 exportations et leurs signatures, aucune importation.
- **La page**, jouée dans un faux navigateur avec le vrai module : décochée, elle ne lit jamais la densité et dessine exactement
  comme avant ; cochée, sa couche est la carte du module sur la rampe, pixel par pixel (douze couches recalculées en Rust avec
  `piece` et une rampe écrite à part), lue une fois par image, sans lissage, sans hyphe ni vignettage, avec les murs d'avant ;
  elle suit la marche, la pause, le pas de référence, le changement de scénario, « Relancer », une adresse changée et la fin
  de la croissance ; les légendes disent ce qui est dessiné.
- 18 tests ; un juge a joué 158 fautes volontaires dans le code, le module, la page et les scripts : 138 sont tuées ; des 20
  autres, 17 sont sans effet observable (chacune classée avec sa raison, trois ne le sont que parce que les six boîtes sont
  carrées), 2 ne le sont que sur les colonies jouées (la case la plus dense n'est jamais un coin), et 1 affaiblit un script
  de test sans changer ce qu'il écrit.

Ce que les tests **ne** vérifient **pas** : que la carte se calcule à la demande plutôt que tout de suite (les valeurs sont les
mêmes) ; la page dans un vrai navigateur (un faux navigateur seulement : la règle CSS qui cache une légende, les couleurs à
l'écran, le temps d'une image) ; des boîtes non carrées (les six scénarios sont carrés).

## Les limites

- La densité est une **longueur d'hyphe par case**, pas une masse, une épaisseur ni une porosité (voir M0).
- **L'échelle suit le maximum du moment** : le jaune est la case la plus dense de l'image affichée. Deux images (deux pas, deux
  graines, deux scénarios) ne se comparent donc pas entre elles par leurs couleurs : un même jaune peut valoir 3 ou 300
  longueurs d'hyphe.
- La racine de la rampe étire les faibles densités : la carte dit où passe l'hyphe plus qu'elle ne mesure.
- La page n'est éprouvée que dans un **faux navigateur** (le juge l'a aussi rendue une fois dans un vrai moteur, Chromium) ;
  Firefox et Safari n'ont pas été essayés.
- Lire 25 921 cases par image coûte environ 2 ms sous Node (mesure du prototype) ; le temps réel d'une image dans un navigateur
  n'a pas été mesuré.

# Le calage sur des chiffres publiés (étape 12, M2)

Cette étape **met en regard** : elle écrit une échelle, mesure la longueur d'hyphe par pointe de la colonie et ses taux de
croissance, règle deux profils de réglages sur des chiffres publiés, et dit où le banc les retrouve et où il ne les retrouve pas.
Elle ne change rien au noyau : `croissance.rs`, `simulation.rs`, `substrat.rs`, `voisinage.rs` sont ceux d'avant, et les
empreintes de la V0 (figure 3), de T0 à T4, de M0 et de M1 sont celles du dépôt, à l'octet. Elle ne promet ni la ramification
selon la place (un modèle à nous, rangé avec M4), ni une **échelle de temps** (des micromètres par heure : un pas de temps n'a
pas de durée ici, les taux se disent par pas), ni la page de la vitrine, ni la résistance d'un vrai matériau. Le compromis entre
vitesse et densité de Schoder et coll. (2024) n'y est qu'une phrase : leur densité est **qualitative** (une échelle à quatre
niveaux, à l'œil : **aucune densité chiffrée**), et rien de Schoder n'entre dans un test.

## Les sources, et leur statut

Le mot de la colonne « Statut » dit ce que nous en avons lu : **lu à la source** (l'image des pages ou le texte intégral),
**de seconde main** (rapporté par quelqu'un d'autre, ou le résumé seul), **choix** (une valeur que nous écrivons, que la source ne
donne pas comme une mesure).

| Source | Statut | Ce qu'on y prend |
|---|---|---|
| Held, Kašpar, Edwards et Nicolau 2019, *PNAS* 116(27) | **choix** | « 5 à 7 µm », le diamètre d'une hyphe de *Neurospora crassa* (Méthodes), et « ∼7 µm », une valeur de normalisation (légende de la fig. 2), sans écart ni effectif : **une case = 7 µm** est un choix, le haut de la fourchette |
| Trinci 1974, *J. Gen. Microbiol.* 81 | **lu à la source** (l'image des pages : le PDF est un scan) | le tableau I (les taux), le tableau II (l'extension par pointe), la définition de la longueur par pointe (longueur totale ÷ nombre de pointes, p. 226), la courbe de la fig. 4 (*Mucor*) |
| Bull et Trinci 1977, *Adv. Microb. Physiol.* 15, par le tableau de la page de David Moore | **de seconde main** : le chapitre n'est pas lu | la fourchette « 35 à 682 µm » (21 espèces, 25 °C) ; seul le tableau est attribué à Bull et Trinci, la phrase est de Moore ; aucune confirmation indépendante |
| Thykaer et coll. 2009 | **de seconde main** : le résumé seul | *P. chrysogenum*, 47 µm par pointe, qui recoupe les 48 µm du tableau de Moore |
| Schoder et coll. 2024, *Mycobiology* 52(2) | **lu à la source** (texte intégral ; figures non lues) | une phrase : il faut trouver un équilibre entre la densité du mycélium et sa vitesse ; qualitatif, entre espèces |

| Quoi | Valeur | Statut |
|---|---|---|
| Échelle | une case = **7 µm** | **choix** (Held 2019) |
| Taux spécifiques par heure, longueur α_h / pointes α_t (tableau I) | *A. nidulans* 0,25 / 0,23 ; *G. candidum* 0,43 / 0,40 ; *M. hiemalis* 0,60 / 0,56 ; *P. chrysogenum* 0,27 / 0,28 ; *N. crassa* spco 1 0,34 / 0,38 (25 °C) et 0,55 / 0,52 (37 °C) | **lu à la source** (relu sur l'image par deux lecteurs et par le juge) |
| Plus grand écart relatif des deux taux | 0,04 / 0,34 = **0,118** (*N. crassa*, 25 °C), le taux de la longueur au dénominateur ; tolérance des tests : **0,12** | calculé sur le tableau I |
| Longueur par pointe de *M. hiemalis* | palier de la fig. 4 : **354 µm** (trois lectures de la courbe : 350, 354, 355) | **lu à la source**, mais sur une courbe en échelle log : aucune valeur n'est écrite dans l'article |
| Longueur par pointe de *P. chrysogenum* | **47 à 48 µm** | **de seconde main** (Thykaer 47, Moore 48), recoupé |
| Fourchette des longueurs par pointe | **35 à 682 µm** | **de seconde main** : un garde-fou large, jamais une cible |

## Ce que fait le code

**`src/calage.rs`** lit les relevés d'une simulation jouée : `longueur_par_pointe_cases` (`biomasse / pointes`, `None` sans
pointe) et `en_micrometres` (× 7), `taux_de_croissance` (la pente de la droite des moindres carrés de `logarithme(valeur)` en
fonction du pas, entre deux indices inclus), `temps_de_doublement` (`ln 2 / taux`), `ecart_relatif` (`|taux des pointes − taux
de la biomasse| / taux de la biomasse`, le dénominateur de Trinci), `fenetre_de_croissance` et `longueur_par_pointe_moyenne` (la
moyenne des `biomasse / pointes` des relevés de la fenêtre). Sans fonction de `f64` du système : le logarithme est
`mesures::logarithme`.

**La fenêtre de croissance.** Trinci mesure de jeunes mycéliums, pendant que les pointes se forment sans arrêt. La fenêtre va du
**premier relevé qui a au moins 40 pointes** jusqu'au relevé **juste avant le premier qui en a plus de 200**. Entre les deux, le
nombre de pointes peut repasser sous 40 (il monte par à-coups et perd une pointe à chaque anastomose) : la première définition,
« le premier intervalle entre 40 et 400 pointes », coupait ainsi la fenêtre de 4 à 6 graines sur 20 du profil rapide après un ou
deux relevés. **Une colonie qui ne dépasse pas 200 pointes pendant la durée jouée n'a pas de fenêtre** (`None`) : une colonie
encore jeune à la fin, ou éteinte dans une cavité fermée, ne mesure pas une phase de croissance. Le haut est 200 et non 400 : plus
près des mycéliums de 10 à 90 pointes de Trinci, et une boîte de 601 × 601 suffit au profil rapide.

**Les deux profils** sont `couplage.txt` avec un autre `intervalle` de ramification (chaque pointe se ramifie à chaque pas avec la
probabilité `1 / intervalle`) et une autre durée (le rapide a aussi une autre boîte et une autre spore) ; l'anastomose (rayon 0,5, délai 10) est celle de
`couplage.txt`.

| Profil | Ancre | `intervalle` | Boîte | Durée |
|---|---|---|---|---|
| `calage-rapide.txt`, rapide et clairsemé | *M. hiemalis*, 354 µm (50,6 cases) ; bande ± 15 % : 300,9 à 407,1 µm | **32** | 601 × 601, spore au centre | 380 |
| `calage-dense.txt`, lent et dense | *P. chrysogenum*, 47 à 48 µm (6,8 cases) ; bande : 40,0 à 55,2 µm | **7,5** | 161 × 161, spore au centre | 100 |

L'intervalle est réglé sur **cent graines** (1 à 100), pas sur les vingt des tests : le rapide à 32 donne 348,9 µm en moyenne
(−1,4 % de l'ancre) ; le dense à 8 en donnait 52,2 (+10 %, alors que les graines 1 à 20 seules donnaient 49,8), à 7 il en
donne 45,2, à **7,5 il en donne 48,4** (+1,9 % du milieu de l'ancre). La bande est l'ancre à ± 15 %, l'incertitude de lecture
des courbes de Trinci : jamais plus étroite.

**L'exemple** `cargo run --release --example mesure_calage [-- <première graine> <dernière graine> [rapide|dense]]` joue chaque
graine jusqu'à la fermeture de sa fenêtre (la suite ne changerait rien aux mesures) et écrit, par graine, la fenêtre, la longueur
par pointe en µm, les deux taux et leur écart, puis le résumé du profil. Sans argument : les graines 1 à 20 des deux profils,
dix secondes de calcul environ ; c'est le tableau de `tests/etape_12/tableau.txt`.

## Ce qu'on mesure (graines 1 à 20)

| | Rapide (ancre 354 µm) | Dense (ancre 47 à 48 µm) |
|---|---|---|
| Longueur par pointe, moyenne | **348,9 µm** | **46,8 µm** |
| Écart-type d'une graine à l'autre | 47,6 µm | 4,0 µm |
| Plus petite, plus grande | 281,1 ; 429,5 µm | 40,3 ; 57,4 µm |
| Dans le garde-fou de 35 à 682 µm (de seconde main) | 20 sur 20 | 20 sur 20 |
| Fenêtre | 92 à 151 relevés | 11 à 16 relevés |
| Taux moyen par pas : biomasse ; pointes | 0,0190 ; 0,0135 | 0,1287 ; 0,1231 |
| **Écart relatif des taux moyens** | **0,288** | **0,043** |
| Écart de chaque graine : moyenne ; plus grand ; graines au-dessus de 0,12 | 0,283 ; 0,451 ; 18 | 0,075 ; 0,192 ; 4 |

Le rapide est au-dessus du dense pour chaque graine (plus de deux fois). Sur cent graines, les nombres tiennent : le rapide 348,9
µm et 0,236 d'écart des taux moyens (trois graines, dont la 55, n'ont pas de fenêtre en 380 pas, et la plus éloignée arrive à
295 cases du centre pour 300 de demi-boîte), le dense 48,4 µm et 0,029.

## Le résultat que le banc ne reproduit pas

**Trinci écrit que la longueur d'hyphe et le nombre de pointes croissent « à peu près au même taux » : au plus 0,118 d'écart
relatif sur son tableau I. Le profil dense le reproduit en moyenne (0,043 sur les graines 1 à 20, 0,029 sur cent) ; le profil
rapide ne le reproduit pas (0,288 sur les graines 1 à 20, 0,236 sur cent ; 18 graines sur 20 et 87 sur 97 dépassent 0,12).**
André a tranché : l'échec est dit tel quel. La tolérance n'est pas relâchée ; un test fige l'échec et se retournera le jour où
l'écart repassera sous 0,12. Même le dense ne la tient pas graine par graine (4 graines sur 20) : sa fenêtre n'a que 11 à 16
relevés.

**D'où ça vient.** Dans le profil rapide, une ramification sur deux (de 41 à 64 % selon la graine) finit en anastomose : la
pointe meurt, l'hyphe reste. La longueur croît du nombre de pointes vivantes à chaque pas ; elle garde donc la mémoire des pas où
les pointes croissaient plus vite. Or le taux des pointes **baisse** dans la fenêtre (pour 16 graines sur 20, il est plus bas
dans la seconde moitié que dans la première) : plus la colonie se remplit, plus les pointes rencontrent une hyphe. La longueur
par pointe monte alors d'environ 35 à 60 cases d'un bout à l'autre de la fenêtre, et les deux taux s'écartent. Une colonie plane
qui ne grandit plus que par son bord a des pointes qui croissent comme son tour et une longueur comme sa surface : un écart qui
tend vers 0,5. Les jeunes mycéliums de Trinci n'en sont pas là. Dans le dense, l'anastomose ne touche que 0 à 7 % des
ramifications de la fenêtre.

**Deux replis mesurés pour André, non retenus et figés dans aucun test** :

- **compter les pointes formées** (`1 + ramifications`) : 0,023 (rapide) et 0,028 (dense) d'écart des taux moyens. Mais ce
  n'est pas la propriété de Trinci : dans ce modèle, chaque pas de chaque pointe vivante ajoute la même longueur et la même chance
  de ramifier, donc le nombre de ramifications suit la biomasse divisée par l'intervalle **par construction**. L'accord ne dit
  rien des pointes.
- **un profil rapide sans la ligne `anastomose`**, à `intervalle` 55 (boîte 801 × 801, durée 600 : la colonie va jusqu'à 339
  cases du centre) : 359,1 µm, 0,014 d'écart des taux moyens, 0,074 en moyenne par graine, 6 graines sur 20 au-dessus de 0,12 ;
  sur cent graines 355,3 µm, 0,003, 0,079 et 26. Sans anastomose, aucune pointe ne meurt : le modèle devient une ramification
  pure, où l'égalité des taux est attendue.

## Ce que les tests vérifient

- **`etape_12_calage.rs`** (exact, sans simulation) : l'échelle et les constantes ; `biomasse / pointes` ; le taux de suites
  géométriques à 1e-9, sur une plage aux deux bouts inclus, les moindres carrés et non la pente des deux bouts, contre une formule
  écrite à part ; le temps de doublement et les cinq temps de Trinci ; l'écart relatif et les six lignes du tableau I ; la fenêtre à
  la main et sur 4 000 suites au hasard contre une machine à états ; la moyenne d'une fenêtre ; les refus ; ni fonction interdite ni
  `HashMap` ; cette section (les cinq sources et leur statut, les chiffres, les nombres du tableau, l'échec).
- **`etape_12_profils.rs`** (les graines 1 à 20 des deux profils, jouées une fois, quatre secondes sur la VM, dix secondes de
  calcul) : les fichiers ne diffèrent de `couplage.txt` que par l'intervalle et la durée (le rapide aussi par la boîte et la spore) ;
  chaque fenêtre est
  refermée loin du bord ; la moyenne dans l'ancre ± 15 %, chaque graine dans le garde-fou, le rapide au-dessus du dense ; le dense
  dans la tolérance en moyenne ; **l'échec du rapide** et sa cause ; le tableau de l'exemple, au caractère ; les mesures contre
  des oracles ; le déterminisme ; arrêter le jeu à la fermeture ne change rien ; les refus de l'exemple. Les tests appellent les
  fonctions de l'exemple, inclus comme un module : jamais un binaire périmé.
- Ils **ne vérifient pas** : d'autres graines que 1 à 20 (les cent graines ci-dessus sont une mesure du juge) ; que les ancres
  sont justes (elles sont lues sur une courbe ou de seconde main) ; la biologie ; les deux replis.

## Les limites

- **Une case ne se mesure pas** : 7 µm est un choix (le haut de 5 à 7 µm, une hyphe de *Neurospora crassa*), et Trinci mesure
  d'autres espèces et d'autres souches. Aucune échelle de temps : on compare des rapports, pas des « par heure ».
- **Aucune densité chiffrée dans Schoder** : son compromis vitesse/densité reste une phrase.
- **La fourchette 35 à 682 µm est de seconde main** (le chapitre de Bull et Trinci n'est pas lu), **sans confirmation
  indépendante** : les mesures indépendantes lues (90 à 445 µm) y tombent, sans la confirmer. Et 354 µm n'est écrit nulle part :
  il se lit sur une courbe.
  Le garde-fou 35 à 682 µm entre dans un test (chaque graine doit y tomber) bien que de seconde main : au sens strict de la règle de
  `FEUILLE-DE-ROUTE.md` (aucun chiffre « à vérifier » dans un test sans relecture à la source), c'est une exception, que l'humain peut
  retirer. La vérification graine par graine tient en une ligne de `tests/etape_12_profils.rs` ; les deux bornes restent aussi dans
  des constantes de `src/calage.rs` et dans le tableau de référence `tests/etape_12/tableau.txt`.
- **Le chiffre dépend du milieu et de la souche** : pour *Mucor hiemalis*, la courbe de Trinci 1974 (gélose au malt, 25 °C) donne
  environ 354 µm, le tableau de Moore (de seconde main, milieu glucosé, 25 °C) 95 µm. L'ancre du profil rapide est la première, lue
  à la source ; un chiffre de longueur par pointe ne se cite jamais sans son milieu.
- **La longueur par pointe d'une colonie qui sature n'est pas stable** : dans le profil rapide, elle monte tout au long de la
  fenêtre (de 35 à 60 cases environ) ; la moyenne de la fenêtre tombe sur l'ancre de Trinci, qui est, elle, un palier. Elle dépend
  donc du choix de la fenêtre (40 à 200 pointes).
- **Un calage, pas une calibration** : un seul réglage, l'intervalle, est ajusté ; d'une graine à l'autre, la longueur varie de
  14 % (rapide) et de 9 % (dense).
- **Le banc ne prédit pas la résistance d'un vrai matériau.**

# Le calcul de raideur : les barres (étape 13, M3)

Cette étape pose un **calcul de raideur d'un réseau de nœuds et de barres articulées**, dans une couche à part
(`src/systeme.rs`, `src/raideur.rs`), et le vérifie sur des cas d'école dont la réponse est exacte. Elle ne change rien au
noyau : `croissance.rs`, `simulation.rs`, `substrat.rs`, `voisinage.rs` sont ceux d'avant, et les empreintes de la V0 à M2 sont
celles du dépôt, à l'octet. Rien n'est relié à la colonie (ce sera l'étape 15). Le réseau est **abstrait** : des nœuds, des
barres de raideur `EA`, des unités au choix de l'appelant, un calcul linéaire en petits déplacements ; **le banc ne prédit pas la résistance d'un vrai matériau**.

## Les sources, et leur statut

**Lu à la source** : la formule est dans un texte relu. **Dérivé** : calculé à la main, sans source publiée ; chaque valeur
dérivée a été recalculée en fractions exactes (avec `√2` et `√3` à part), à part du dépôt.

| Quoi | Source | Statut |
|---|---|---|
| Raideur d'une barre : `EA/L`, matrice `(EA/L) [[1, −1], [−1, 1]]` | Felippa, *Introduction to Finite Element Methods*, ch. 11, éq. 11.26 ; cours de Kharagpur (NPTEL, *Structural Analysis*, leçon 30, éq. 30.1a) ; MIT 1.050, ch. 5 | **lu à la source** (Kharagpur relu par le juge de l'étape ; les deux autres par ses lecteurs) |
| Treillis triangulaire de barres articulées : `E* = ρ* Es/3`, `ν* = 1/3`, avec `ρ* = 2√3 t/l` : `E* = (2/√3) EA/l` | MIT 3.054 (Gibson), leçon 4, page 25 : « exact calculation: E = 1.15 Es (t/l) for equilateral triangles » ; Omidi et St-Pierre 2022, *Mater. Des.* 213, tableau 1 | **lu à la source** (MIT relu par le juge ; Omidi par ses lecteurs) ; le lien entre `ρ*` et la barre est calculé |
| Série (`1/k = 1/k₁ + 1/k₂`), parallèle (`k = k₁ + k₂`), deux barres symétriques, triangle équilatéral, carrés contreventés | aucune source ouverte : des exercices de statique | **dérivé** |

## Ce que fait le code

- **`raideur`** : des `Noeud`s, des `Barre { a, b, ea }`, des `Appui`s (un déplacement imposé sur un axe d'un nœud : zéro pour
  un appui, non nul pour un bord que l'on pousse), des `Charge`s (des forces aux nœuds). Une barre de longueur `L` et de
  direction `e = (dx/L, dy/L)` s'allonge de `δ = e·(u_b − u_a)` et porte `N = EA δ/L` (**positif en traction**) ; sa matrice
  est `(EA/L) g gᵀ` avec `g = (−e, e)`. `resoudre` rend les déplacements, les **réactions** (ce que l'appui exerce sur la
  structure, `K u − f` aux degrés de liberté imposés), l'effort de chaque barre et l'énergie `½ uᵀ K u`. Une donnée impossible
  (indice hors limites, longueur nulle, `ea` nul, négatif ou non fini, coordonnée ou valeur non finie, appui répété) rend
  `DonneeInvalide`.
- **`systeme`** : une **matrice pleine**, la décomposition de Cholesky, après **mise à l'échelle par la diagonale** (chaque
  inconnue ramenée à une raideur un). `n³/3` opérations : quelques milliers d'inconnues au plus.
- Seulement `+ − × ÷` et `sqrt` : le même résultat, bit à bit, sur toutes les machines.

## Les cas d'école

`cargo run --release --example cas_barres` écrit les 45 valeurs (calculée, exacte, erreur relative, statut). La plus grande
erreur relative est de 1,5e-15 ; les tests demandent 1e-12.

| Cas | Valeurs exactes | Statut |
|---|---|---|
| Une barre (`L = 2`, `EA = 3`, tirée de 5) | déplacement `F L/EA = 10/3`, effort 5, réaction −5, énergie 25/3 | lu à la source |
| Deux barres en série (raideurs 2 et 3/2, tirées de 1) | déplacements 1/2 et 7/6 | dérivé |
| Deux barres en parallèle (`EA` = 2 et 3, `L = 2`, tirées de 1) | déplacement 2/5, efforts 2/5 et 3/5 | dérivé |
| Deux barres symétriques 3-4-5 (pieds en (−3, 0) et (3, 0), sommet (0, 4) chargé de (0, −1), `EA = 1`) | effort **−5/8** dans chaque barre, déplacement vertical **−125/32**, horizontal nul, réactions 3/8 et 1/2 | dérivé |
| Triangle équilatéral de côté 1 (`EA = 1` ; nœud 0 fixe, nœud 1 sur rouleau, sommet tiré de 1 selon `x`) | efforts 1/2 (base), −1, +1 ; réactions −1, −√3/2, +√3/2 ; déplacements 1/2 (rouleau), 9/4 et **−√3/12** (sommet) ; énergie 9/8 | dérivé |
| Carré de côté 1 et sa diagonale 0-2 (`EA = 1` ; nœud 0 fixe, nœud 1 tenu en `y`, nœud 3 tiré de 1 selon `x`) | diagonale √2, côté 1-2 : −1 ; déplacements `1 + 2√2`, −1, `2 + 2√2` ; énergie `1 + √2` | dérivé |
| Le même à deux diagonales, nœuds 0 et 1 fixes (hyperstatique) | diagonales `(−4 + 13√2)/23` et `(−4 − 10√2)/23` ; côtés `(−13 + 2√2)/23`, `(10 + 2√2)/23` ; déplacements `(5 + 24√2)/23`, `(−13 + 2√2)/23`, `(18 + 22√2)/23`, `(10 + 2√2)/23` ; énergie `(9 + 11√2)/23` | dérivé |
| Treillis triangulaire infini (`l = 1`, `EA = 1`) | `E* l/EA = (2/√3)` = 1,1547 ; `ν* = 1/3` | lu à la source |

Le treillis est celui d'un **réseau infini** : on le calcule sur l'étoile d'un nœud libre et de ses six voisins, déplacés de la
déformation moyenne (chaque barre de l'étoile compte pour moitié). Un test à part prend un morceau fini du treillis, déplace
son bord d'un champ affine quelconque `u = G x`, et vérifie que l'intérieur suit ce champ et que chaque barre de direction `e`
porte `EA eᵀ G e` (de 2 à 722 inconnues, à 1e-11 près).

## Le mécanisme : le plus petit mode, pas le plus petit pivot

Un **mécanisme** (un carré sans diagonale, une barre non tenue en travers, trop peu d'appuis, un nœud sans barre dont un déplacement n'est pas
imposé) rend la
matrice singulière : **un système singulier ne rend jamais un nombre**, il rend `Mecanisme`. En `f64`, « singulière » demande
une mesure et un seuil.

**Le plus petit pivot ne convient pas.** Mesuré par le prototype de l'étape et rejoué par son juge, sur 20 000 grilles de
barres au hasard par ligne (de 2 × 2 à 6 × 6 nœuds,
`EA = 10^k`, `k` de 0 au contraste ; valides avec trois appuis, mécanismes avec deux) : le plus petit pivot d'un réseau valide
descend avec le contraste des raideurs (2e-3 sans contraste ; 9e-9 à 1e6 ; 1e-11 à 1e9) pendant que celui d'un mécanisme, qui devrait être nul,
monte avec lui (4e-14 sans contraste ; 3e-9 à 1e6 ; 5e-7 à 1e9). Ils se croisent entre 1e6 et 1e7 : au-delà, aucun seuil sur le pivot ne sépare
un réseau valide d'un mécanisme.

**Le plus petit mode convient.** C'est la plus petite valeur propre de la matrice mise à l'échelle, estimée par deux pas
d'itération inverse (le vecteur de départ pèse chaque inconnue par l'inverse de son pivot). Mesuré sur 2 000 réseaux par ligne
(les mêmes grilles ; un mécanisme : un appui en moins, ou un nœud de plus pendu à une seule barre) :

| Contraste des raideurs | 1 | 1e4 | 1e6 | 1e8 | 1e9 | 1e10 | 1e11 | 1e12 | 1e15 |
|---|---|---|---|---|---|---|---|---|---|
| plus petit mode valide | 1,7e-4 | 1,1e-7 | 3,2e-9 | 2,3e-11 | 6,9e-12 | 7,1e-13 | 2,6e-14 | 8,8e-15 | 0 |
| plus grand mode de mécanisme | 5,0e-16 | 5,6e-16 | 4,5e-16 | 5,3e-16 | 4,7e-16 | 5,6e-16 | 4,4e-16 | 4,4e-16 | 5,2e-16 |

Le mode d'un mécanisme reste un bruit d'arrondi, quel que soit le contraste : au plus 6,2e-16 sur ces grilles (poutres de
l'étape 14 comprises), au plus 9e-17 pour des grilles de 12 × 12 à 28 × 28 nœuds. Un relecteur, avec le générateur des tests
(raideurs de `1,5 ± 0,5` fois `10^k`) et un portage Python du code, a trouvé jusqu'à 5,3e-15 pour un contraste de 1e13 à 1e15
(toujours un appui en moins) : ce qui reste est de l'ordre de 1e-16 à 5e-15. **`SEUIL_SINGULIER` = 1e-12** : de 190 à 1 600 fois
ce bruit. Un réseau valide est
accepté tant que son mode dépasse le seuil, et **l'erreur relative de son résultat vaut au plus 2,5e-16 divisé par son mode**
(mesuré sur deux ressorts en série de raideurs 1 et `r`, `r` de 1 à 1e11, sur des consoles de 1 à 600 poutres et sur le toit
de l'étape 14 ; ce n'est pas un théorème : les tests permettent dix fois plus) : trois à
quatre chiffres justes au seuil, sept au moins pour un contraste de 1e6. Un réseau valide refusé (un contraste de plus de 1e10 environ,
ou plus de 800 poutres alignées bout à bout) est si mal conditionné que le calcul ne le distingue plus d'un mécanisme : le
refus est alors la réponse honnête.

## Ce que les tests vérifient

- **`etape_13_solveur.rs`** : Cholesky contre une élimination de Gauss écrite à part (200 matrices de 1 à 30 inconnues), le
  résidu, l'indifférence aux unités (échelles de 1e-6 à 1e6) ; dix matrices singulières écrites à la main et cent au hasard :
  jamais un nombre ; le plus petit mode (valeurs connues, la mise à l'échelle), le seuil à un facteur 4, l'erreur de deux
  ressorts en série sous `2,2e-15 / mode` ; les degrés de liberté imposés, les réactions, l'énergie, les paniques.
- **`etape_13_reseaux.rs`** (cent réseaux au hasard par test, tolérance 2e-11) : la matrice (une barre à la main, symétrique au
  bit près, assemblée à part, sans force pour une translation ou une rotation d'ensemble) ; `K u = f + R`, l'oracle de Gauss,
  l'équilibre des forces et des moments, l'effort recalculé, l'énergie de trois façons ; la superposition, la réciprocité de
  Maxwell, le corps rigide, l'échelle, l'ordre des nœuds et le sens des barres, deux exécutions égales au bit près ; les
  mécanismes à la main et sur 240 réseaux de contraste 2 à 2e15 (un appui en moins, un nœud pendu) ; les données invalides.
- **`etape_13_cas.rs`** : les 45 valeurs à 1e-12, leurs statuts, cinq valeurs redites en clair ; le morceau de treillis ; ni
  fonction interdite ni `HashMap`, le noyau n'importe pas la raideur ; cette section.

## Ce que les tests ne vérifient pas

- Un réseau de plus de 722 inconnues ; un réseau venu d'une colonie ; le temps de calcul.
- Les valeurs « dérivées » ne sont comparées à aucune publication : elles ont été recalculées en fractions exactes par deux
  calculs indépendants du code (celui du prototype, celui du juge), c'est tout.
- Les deux tableaux de mesure ci-dessus viennent des sondes du prototype et du juge (2 000 et 20 000 réseaux par ligne), qui
  ne sont pas dans le dépôt ; les tests en rejouent 240 réseaux, et les chaînes.
- Le seuil est mesuré sur des grilles et des chaînes : un réseau d'une autre famille peut avoir un mode plus petit pour un
  même contraste (il serait refusé, pas faux).
- Windows : le déterminisme repose sur l'absence de fonction de la bibliothèque mathématique, pas sur un essai des deux machines
  avant la CI.

## Les limites

- **Des grandeurs dans la plage normale de `f64`** : une barre plus courte que 1,5e-154 environ (le carré de sa longueur est sous le
  plus petit nombre normal) peut rendre des efforts faux sans refus (un triangle réduit à 2e-162 : 11 % d'écart) ; une raideur
  `EA/L` qui déborde est classée `Mecanisme` ; une raideur ou une charge sous-normale peut rendre des nombres non finis. Rien
  de réaliste n'approche ces bornes, mais elles ne sont pas contrôlées (mesuré par un relecteur, avec un portage Python).
- **Un réseau abstrait, pas un matériau** : ni section, ni module, ni rupture, ni flambement ; le banc ne prédit pas la
  résistance d'un vrai matériau.
- **Petits déplacements, comportement linéaire** : la raideur est celle de la forme de départ. Un vrai mycélium est humide, mou
  et non linéaire.
- **Des barres articulées** : aucune flexion (c'est l'étape 14) ; un réseau peu connecté est un mécanisme.
- **Une matrice pleine** : mémoire en `n²`, temps en `n³` ; le réseau d'une colonie entière demandera une autre méthode.
- **Le treillis de la littérature est un réseau infini** ; un morceau fini chargé par ses bords s'en écarte.

# Les poutres (étape 14, M3)

Cette étape ajoute au calcul de raideur les **poutres à nœuds rigides** (`src/poutres.rs`) : une poutre s'étire (`EA`) **et
fléchit** (`EI`), et un nœud a une rotation que ses poutres partagent. Le solveur est celui de l'étape 13 ; le noyau ne change
pas, et rien n'est relié à la colonie. C'est le modèle d'**Euler-Bernoulli**, linéaire, en petits déplacements, sur un réseau
abstrait : **le banc ne prédit pas la résistance d'un vrai matériau**.

## Les sources, et leur statut

Les statuts sont ceux de l'étape 13 : **lu à la source**, ou **dérivé** (calculé à la main, recalculé en fractions exactes à
part du dépôt).

| Quoi | Source | Statut |
|---|---|---|
| La matrice 6 × 6 de la poutre (ordre `u₁, v₁, θ₁, u₂, v₂, θ₂`) et sa rotation `K = Tᵀ k T` | cours de Kharagpur (NPTEL, *Structural Analysis*, leçon 30 : éq. 30.1a, 30.2 à 30.4, 30.10, et les nombres de l'exemple 30.1) ; MIT 13.122, notes 36 (le bloc 4 × 4 de flexion) ; Politechnika Warszawska FEM1 10B ; CALFEM `beam1e` | **lu à la source** (Kharagpur et MIT relus par le juge de l'étape ; les deux autres par ses lecteurs) |
| La rotation `θ = dv/dx`, positive dans le sens **anti-horaire** | Kharagpur, leçon 27, § 27.2 : les moments et les rotations sont positifs dans le sens anti-horaire | **lu à la source** (relu par le juge) |
| Console : flèche `P L³/(3 EI)`, rotation `P L²/(2 EI)` ; charge répartie : `q L⁴/(8 EI)`, `q L³/(6 EI)` | MIT 1.050 (Bucciarelli), ch. 10, p. 268 ; Felippa, ex. 12.1 | **lu à la source** (par les lecteurs ; non relu par le juge) |
| Poutre sur deux appuis, charge au milieu : `P L³/(48 EI)` | Roylance, MIT 3.11, éq. 7 | **lu à la source** (par les lecteurs ; non relu par le juge) |
| Poutre encastrée aux deux bouts : moments `P a b²/L²` et `P a² b/L²`, donc `P L/8` pour une charge au milieu | Kharagpur, leçon 27, éq. 27.8 | **lu à la source** (relu par le juge) |
| Poutre encastrée aux deux bouts : flèche `P L³/(192 EI)` | aucun texte ouvert | **dérivé** |
| Nid d'abeille, flexion seule : `E₁* = E₂* = (4/√3) (t/l)³ Es = 2,31 (t/l)³ Es`, `ν* = 1` ; hypothèses : `t/l` petit, étirement et cisaillement des parois négligés | MIT 3.054 (Gibson), leçon 4, pages 8, 11, 12 et 15 | **lu à la source** (relu par le juge, sur le texte du PDF) |
| Nid d'abeille, avec l'étirement des parois : `E* = (4/√3) (t/l)³ Es / (1 + 3 (t/l)²)`, `ν* = (1 − (t/l)²)/(1 + 3 (t/l)²)` | aucune source lue ne l'écrit ainsi (Pham et Hütter 2021 donnent le coefficient 3, de seconde main) | **dérivé** (trois calculs en fractions exactes dans ℚ(√3) : un vérificateur, le prototype, le juge) |
| Portiques, toit en poutres, moment au bout | aucune source | **dérivé** |

## Ce que fait le code

- Trois degrés de liberté par nœud, `[u, v, θ]` : l'axe 2 d'un `Appui` est la rotation, celui d'une `Charge` un **moment**
  (anti-horaire positif). `Poutre { a, b, ea, ei }` : `ea` et `ei` strictement positifs (sans flexion, c'est une barre).
- La matrice de la poutre dans son repère, avec `a = EA/L` et `b = EI/L³` : `a` et `−a` pour l'étirement ; `12b`, `6bL`,
  `4bL²`, `2bL²` pour la flexion (le tableau complet est dans `src/poutres.rs`). Dans le repère du réseau : `K = Tᵀ k T`,
  `T = diag(R, R)`, `R = [[c, s, 0], [−s, c, 0], [0, 0, 1]]`, où `c = dx/L` et `s = dy/L` : aucune fonction trigonométrique.
- `resoudre` rend les déplacements, les réactions (forces et moments), l'énergie, et pour chaque poutre ses six **efforts
  d'extrémité** dans son repère, `[N_a, V_a, M_a, N_b, V_b, M_b]` : ce que les nœuds exercent sur elle ; `N_b` est l'effort
  normal, positif en traction. Une console encastrée en `a` et chargée de `P` en travers au bout a `[0, −P, −P L, 0, P, 0]`.
- **La solution est exacte aux nœuds** pour une rigidité constante par poutre et des charges aux nœuds (la flèche exacte est
  un polynôme de degré trois) : une seule poutre suffit pour une console, mais il faut **un nœud à chaque charge** ; une
  charge répartie se donne par ses charges nodales cohérentes (`q L/2` et `± q L²/12`).

## Les cas d'école

`cargo run --release --example cas_poutres` écrit les 61 valeurs. La plus grande erreur relative est de 3e-15, sauf le module
du nid d'abeille (1,1e-13 à `t/l` = 0,02 : il sort d'une soustraction qui perd un facteur `1/(8 (t/l)²)` environ) ; les tests demandent 2e-12.
Partout `EA = 5` et `EI = 3`, sauf mention.

| Cas | Valeurs exactes | Statut |
|---|---|---|
| Console (`L = 2`, `P = 7` au bout) | flèche `P L³/(3 EI)` = 56/9, rotation `P L²/(2 EI)` = 14/3 | lu à la source |
| | moment d'encastrement `P L` = 14, efforts `[0, −7, −14, 0, 7, 0]`, énergie 196/9 ; dressée vers le haut et poussée vers la gauche : la même, rotation **positive** | dérivé |
| Console étirée de 7 | allongement `F L/EA` = 14/5, effort normal 7 | lu à la source |
| Console, moment 7 au bout | flèche `M L²/(2 EI)` = 14/3, rotation `M L/EI` = 14/3 | dérivé |
| Console, charge répartie `q = 7` | flèche `q L⁴/(8 EI)` = 14/3, rotation `q L³/(6 EI)` = 28/9 | lu à la source |
| Poutre sur deux appuis (`L = 4`, `P = 7` au milieu) | flèche `P L³/(48 EI)` = 28/9 | lu à la source |
| | rotation des appuis `P L²/(16 EI)` = 7/3, moment au milieu `P L/4` = 7 | dérivé |
| Poutre encastrée aux deux bouts (`L = 4`, `P = 7` au milieu) | moments d'encastrement `P L/8` = 7/2 | lu à la source |
| | flèche `P L³/(192 EI)` = 7/9, énergie 49/18 | dérivé |
| La même, la charge au quart (`a = 1`, `b = 3`) | moments `P a b²/L²` = 63/16 et `P a² b/L²` = 21/16 | lu à la source |
| | flèche `P a³ b³/(3 EI L³)` = 21/64, réaction 189/32 | dérivé |
| Poutre sur deux appuis (`L = 3`, `EI = 1`), moment 2 à un bout | rotations `M L/(3 EI)` = 2 et −1 : **ce n'est pas un mécanisme** | dérivé |
| Portique en L (montant de 3, traverse de 4, bout chargé de 7) | coin : 42, −21/5, rotation −28 ; bout : −7469/45, rotation −140/3 ; moment d'encastrement 28 ; énergie 52283/90 | dérivé |
| Portique à deux montants (3, 4, 3 ; pieds encastrés ; poussé de 7) | nœud poussé : 298137/50635, 567/494, −371427/202540 ; moments aux pieds 821121/101270 et 530649/101270 ; énergie 2086959/101270 | dérivé |
| Toit 3-4-5 de l'étape 13, en poutres (`EA = EI = 1`, pieds articulés) | flèche −3125/854, effort −250/427 | dérivé |
| Nid d'abeille infini, `t/l` = 0,02 ; 0,1 ; 0,3 | `E*/Es = (4/√3) (t/l)³ / (1 + 3 (t/l)²)`, `ν* = (1 − (t/l)²)/(1 + 3 (t/l)²)` | dérivé |

**Une poutre très souple en flexion rend les barres ; une poutre très raide ne les rend pas.** Le toit 3-4-5 a, en barres, une
flèche de −125/32 et un effort de −5/8. En poutres, le nœud rigide du sommet y ajoute la flexion : la flèche et l'effort sont
divisés par `1 + 27 EI/(400 EA)`. Quand `EI` tend vers zéro on retrouve les barres (à 7e-8 près pour `EI/EA` = 1e-6) ; quand
`EI` grandit, le toit ne bouge plus : un réseau à nœuds rigides et à poutres très raides n'est pas un treillis articulé.

## Le nid d'abeille

Le réseau de la littérature est un **réseau infini** d'hexagones réguliers, de parois de longueur `l` et d'épaisseur `t`
(`EA = Es t`, `EI = Es t³/12` par unité de profondeur). Sa cellule a deux nœuds et trois parois : on la calcule sur l'étoile
d'un nœud libre et de ses trois voisins, déplacés de la déformation moyenne sans tourner. C'est exact pour le réseau infini
(chaque paroi est un axe de symétrie : aucun nœud ne tourne ; les trois voisins sont les images les uns des autres), et le
calcul du juge en fractions exactes sur la cellule périodique complète (rotations et décalage libres) donne les mêmes constantes.
Le module vient de trois énergies : `E* = C₁₁ − C₁₂²/C₂₂`, `ν* = C₁₂/C₂₂`.

| `t/l` | `E*/Es` calculé | écart à la formule avec étirement | écart à la formule publiée (flexion seule) | `ν*` |
|---|---|---|---|---|
| 0,02 | 1,845306e-5 | 1,1e-13 | 0,12 % | 0,998402 |
| 0,1 | 2,242137e-3 | 2e-16 | 2,9 % | 0,961165 |
| 0,3 | 4,909750e-2 | 9e-16 | 21 % | 0,716535 |

- **La formule publiée est retrouvée dans ses hypothèses** : elle néglige l'étirement des parois, et le calcul s'en écarte de
  `3 (t/l)²/(1 + 3 (t/l)²)`, ni plus ni moins ; avec des parois dix mille fois plus raides en étirement, le calcul la rend à
  `3 (t/l)²/10⁴` près, et `ν* = 1` de même.
- **La formule avec étirement est dérivée, pas lue** : elle est exacte pour des poutres d'Euler-Bernoulli à nœuds ponctuels.
  Les corrections de **cisaillement** des parois (un terme de plus au dénominateur, sur lequel deux sources ne s'accordent
  pas : 5,4 + 1,5 ν ou 5,4 + 2,4 ν) ne sont pas faites, et aucune n'entre dans un test.
- La densité relative `(2/√3) (t/l)` du cours ne compte pas le recouvrement des parois aux nœuds (2,9 % d'écart à `t/l` = 0,1) :
  elle ne sert à aucun test.
- **Un panneau fini n'est pas le réseau infini** : le prototype de l'étape a mesuré un panneau de `k × k` cellules serré
  entre deux plaques : +26 % (3 × 3), +8 % (6 × 6), +5 % (10 × 10) à `t/l` = 0,02. Cette mesure n'est pas rejouée par les tests.

## Ce que les tests vérifient

- **`etape_14_cas.rs`** : la matrice (la poutre horizontale à la main, à l'égalité ; l'inclinée 3-4-5 en fractions ; le montant
  de l'exemple 30.1 de Kharagpur aux trois chiffres imprimés, avec son signe ; symétrique au bit près ; aucune force pour une
  translation ou une rotation d'ensemble) ; les 61 valeurs à 2e-12, neuf redites en clair ; le nid d'abeille pour sept `t/l` de
  0,001 à 0,3 (la formule avec étirement à 1e-9, l'écart à la formule publiée, les parois presque inextensibles, l'isotropie) ;
  ni fonction interdite ni `HashMap` ; cette section.
- **`etape_14_reseaux.rs`** (cent réseaux au hasard par test, tolérance 2e-11) : `K u = f + R`, l'oracle de Gauss, l'équilibre
  des forces **et des moments**, du réseau et de chaque poutre, l'effort normal recalculé, l'énergie de trois façons (dont
  `N² L/(2 EA) + L (m_a² + m_a m_b + m_b²)/(6 EI)`) ; la superposition, Maxwell entre forces et moments, le corps rigide
  (translation et rotation), l'échelle, l'ordre et le sens des poutres, deux exécutions égales au bit près ; les mécanismes
  (trop peu d'appuis, un nœud sans poutre, une console de 1 à 100 poutres sans encastrement, 200 réseaux de contraste 2 à 2e12
  avec un appui en moins) et ce qui n'en est pas (le carré de poutres, la console de 300 poutres, à `2,2e-15 / mode` près) ;
  le toit pour `EI/EA` de 1e-9 à 1e9 ; les données invalides.

## Ce que les tests ne vérifient pas

- Le cisaillement des parois, le flambement, une section qui varie le long d'une poutre, une charge entre deux nœuds.
- Un réseau de plus de 903 inconnues ; un nid d'abeille fini ; le module de cisaillement du nid d'abeille.
- Les formules « lues par les lecteurs » (MIT 1.050, Felippa, Roylance) n'ont pas été relues par le juge ; le calcul les retrouve.

## Les limites

- **Euler-Bernoulli** : pas de déformation de **cisaillement** ; elle croît comme le carré de l'élancement : environ 0,8 `(h/L)²`
  de la flexion pour une console, 3 `(h/L)²` pour une paroi tenue aux deux bouts comme celles du nid d'abeille (2,9 % à `t/l` = 0,1,
  26 % à 0,3).
- **Petits déplacements, comportement linéaire, nœuds ponctuels et parfaitement rigides.** Un vrai mycélium est humide, mou et
  non linéaire.
- **Le nid d'abeille de la littérature est un réseau infini**, et sa formule simple vaut pour `t/l` petit.
- **Une longue file de poutres alignées est mal conditionnée** (le plus petit mode d'une console de `n` poutres décroît comme
  `1/n⁴`) : au-delà de 800 environ, elle est refusée. La solution étant exacte aux nœuds, une poutre droite se maille d'un seul
  élément.
- **Des longueurs dans la plage où `L³` est un nombre normal de `f64`** (de 3e-103 à 5e102 environ) : plus courte, une poutre peut
  rendre une flexion fausse sans refus (8 % à 3e-108) ; plus longue, elle est classée `Mecanisme`. Rien de réaliste n'approche ces
  bornes, mais elles ne sont pas contrôlées (mesuré par un relecteur, avec un portage Python).
- **Une matrice pleine** : quelques milliers d'inconnues au plus.

# La raideur d'une colonie (étape 15, M3)

**Ce nombre dépend du choix des mors.** L'étape tire d'une colonie simulée un **réseau de poutres** (`src/reseau.rs`), tient
les nœuds d'un bord, pousse ceux du bord d'en face et lit la **raideur** `K` : la force qu'il faut pour un déplacement donné. Un
mors est ici **une colonne de cases** ; une bande de deux à cinq colonnes donne un `K` de 1 à 15 fois plus grand (mesuré, voir
plus bas), et rien ne dit qu'une largeur est plus juste qu'une autre. D'une graine à l'autre, le même scénario donne des `K` de
0,0022 à 0,53 µN par case (mesuré sur la colonie de démonstration : graines 1 à 120, de 40 à 100 pas de 10 en 10, soit 840 essais, dont
169 traversent la boîte avec au plus 1 000 nœuds ; 606 ne la traversent pas encore et 65 dépassent le plafond ; d'autres graines ou
d'autres pas peuvent sortir de cette fourchette). Le banc **ne prédit pas la résistance d'un vrai matériau** : il calcule la raideur du réseau qu'il a
simulé, avec des raideurs d'hyphe que l'appelant donne. **Aucune raideur de colonie n'est publiée** : il n'y a rien à
reproduire, et aucun nombre de cette étape n'est « lu à la source » en tant que raideur de colonie. Le solveur est celui des
étapes 13 et 14 ; **le noyau ne change pas** (aucun champ, aucun accesseur : les empreintes sont celles du dépôt).

## Les nombres, et leur statut

**Lu à la source** : le nombre est dans un texte relu. **Calculé** : tiré d'un nombre lu par un calcul écrit, sans mesure
publiée. **Choix** : décidé par ce banc. **Dérivé** : calculé à la main ou en fractions exactes, sans source publiée.

| Quoi | D'où | Statut |
|---|---|---|
| `EI` = 18,3 µN·µm² : la raideur en flexion d'une hyphe d'*A. niger* (22 hyphes, une culture, paroi seule) | Brinkmann et coll. 2024, *Frontiers in Bioengineering and Biotechnology*, §3.3, fig. 9E | **lu à la source** (par un lecteur et un vérificateur du pilote ; ni le prototype ni le juge ne l'ont relu ; le supplément n'est lu par personne) |
| `EA` ≈ 22,9 µN : la raideur axiale | le module de la flexion (10,1 MPa, §4 de l'article) **pris isotrope**, fois l'aire de la paroi `π(1,4² − 1,113²)` µm² ; l'article ne donne **aucune** raideur axiale | **calculé** : une extrapolation, pas une mesure de l'article |
| Une case = 7 µm (donc `EI` = 18,3/49 = 0,3735 µN·case²) | M2 | **choix** |
| La règle de contact : une pointe anastomosée est reliée au point le plus proche du segment visé, par une poutre de mêmes `EA` et `EI` | aucune : le noyau ne crée aucun segment pour une anastomose | **choix** |
| Les mors : une colonne de cases par bord, les trois degrés de liberté de chaque nœud imposés | aucune | **choix** (le nombre en dépend) |
| `COURT` = 0,01 case (0,07 µm) : la longueur sous laquelle une poutre est fondue | la mesure ci-dessous | **choix** (il change `K` de quelques millièmes) |
| `K` de la colonie de démonstration : 0,0752 µN par case, soit 0,0107 N/m | ce banc | **calculé** ; aucune raideur de colonie n'est publiée |
| Les raideurs exactes des neuf colonies faites à la main (voir plus bas) | fractions exactes, calculées à part du dépôt par deux solveurs écrits séparément (le prototype, le juge) ; le Z et le pont qui porte aussi par une autre méthode (les forces, sans matrice de raideur) | **dérivé** |

## Ce que fait le code

- **Le réseau** (`extraire`). Une colonie est un **arbre** : les nœuds sont les extrémités de segments **égales bit à bit**
  (une ramification naît au bout du segment de sa mère), une poutre par segment, `EA` et `EI` **uniformes** (`Hyphe`). Il
  prend des segments, pas une simulation : les tests le nourrissent de colonies faites à la main.
- **Les contacts** (`contacts`). Une anastomose ne crée aucun segment : le noyau arrête l'apex près d'un segment étranger et
  ne dit pas lequel. On **rejoue sa recherche** : le même `Index`, rempli dans le même ordre, interrogé juste après chaque
  segment avec les mêmes réglages et la même règle de l'obstacle (`segment_libre`). Ce sont les mêmes appels sur les mêmes
  données : le rejeu est égal au noyau par construction, et les tests le recomptent. « Inactif » n'est pas « anastomosé » (un
  mur arrête aussi des pointes) : c'est la recherche qui les sépare. **Aucun accesseur du noyau n'est nécessaire.**
- **La coupe et le pont.** Le segment visé est **coupé** au point le plus proche de la pointe (un nœud de plus, deux poutres
  alignées : exact pour un segment droit) et un **pont** relie ce point à la pointe. Une pointe qui se projette au-delà d'un
  bout du segment est reliée à ce bout, sans coupe ; plusieurs coupes d'un segment sont rangées le long du segment.
- **La fonte** (`COURT`). Une poutre plus courte que 0,01 case est fondue : ses deux nœuds n'en font plus qu'un. Pour un
  pont, **la pointe rejoint son point d'arrivée** (l'hyphe visée reste droite) ; ailleurs le nœud le plus récent rejoint le
  plus ancien (un point de coupe rejoint le bout du segment). On recommence tant qu'une poutre est trop courte : le réseau
  rendu n'en a aucune. Une pointe **sur** le segment visé ne laisse pas de pont : c'est le cas limite.
- **L'essai** (`pousser`). Deux **mors** rigides : les nœuds de la colonne (ou de la ligne) de cases du bord `de` sont
  encastrés (un nœud `(x, y)` est dans la case `(⌊x⌋, ⌊y⌋)`, la règle du plancher de M0) ; ceux du bord `vers` sont déplacés
  de `delta` selon l'axe qui joint les deux bords, l'autre translation et la rotation à zéro. `K = F/δ`, où `F` est la somme
  des réactions du mors déplacé selon cet axe ; l'énergie est `½ K δ²` ; chaque poutre a ses efforts et son énergie. **Une
  colonie qui ne traverse pas a `K = 0`**, sans calcul et sans erreur.
- **Les refus** (`ErreurReseau`), jamais un nombre à la place : `DonneeInvalide` (`ea`, `ei` ou `delta` qui n'est pas un
  nombre normal ; une coordonnée de plus de 1e50 ; un segment nul ; un contact hors liste ; deux bords qui ne sont pas
  opposés ; une poutre dont le cube de la longueur sort de la plage normale ; un résultat qui déborde ou s'efface),
  `TropGrand` (plus de `INCONNUES_MAX` = 3 000 degrés de liberté, soit 1 000 nœuds, pour une colonie qui traverse),
  `Mecanisme` (celui de `poutres::resoudre`).
- Seulement `+ − × ÷`, `sqrt`, `floor`, `clamp` : le même résultat, bit à bit, sur toutes les machines.

## Les mesures

Mesuré par le prototype, puis refait par le juge sur d'autres graines (les nombres sont ceux du juge, sauf mention).

**Le rejeu de la recherche du noyau.** Autant de contacts que le compteur `anastomoses()` : 330 colonies sur 330 pour le
prototype (onze scénarios, graines 1 à 30) ; pour le juge, 240 sur 240 (sept scénarios aux graines 31 à 60, cinq moules aux
graines 31 à 36 : 320 000 anastomoses) et 1 080 sur 1 080 sur des boîtes à nous (un mur d'une case ou des piliers, des rayons
de 0,5 à 3,5, des délais de 0, 1 et 10, avec et sans thigmotropisme). **La règle de l'obstacle ne change rien aux réglages de
M2** (un mur d'au moins une case est plus épais que le rayon de 0,5) ; elle compte dès que le rayon dépasse l'épaisseur d'un
mur : sans elle, le compte est faux pour 15 de ces 1 080 colonies, toutes à un rayon d'au moins 1 et un délai de 10.

**La fonte.** 93 colonies de la petite boîte (30 × 20, graines 1 à 120, de 40 à 100 pas) qui traversent avec au plus 1 000
nœuds. Sans fonte, **22 sont refusées** comme mécanismes (une pointe à 1e-16 d'une hyphe : la plus courte poutre) ; avec
`COURT` = 0,001, une ; avec 0,003, aucune ; avec **0,01, aucune, et un plus petit mode d'au moins 1,8e-9** (le seuil
du refus est 1e-12). Le choix compte : `K` change d'au plus 3e-3 entre `COURT` = 0,01 et 0,03, et de 1e-2 à 0,1. La règle
« le plus ancien nœud reste » du prototype pliait l'hyphe visée de moins de 0,01 case sous une pointe trop proche : `K` en
changeait jusqu'à 4e-4 (4,5e-6 plus bas pour la graine 8 à 50 pas) ; la règle retenue garde l'hyphe droite.

**La largeur des mors** (`K` en µN par case pour des mors de 1, 2, 3, 4 et 5 colonnes) : graine 42 à 60 pas, 0,0752 ; 0,1057 ;
0,1328 ; 0,1510 ; 0,1613. Graine 8 à 100 pas : 0,00569 ; 0,00570 ; 0,0512 ; 0,0566 ; 0,0703. Graine 38 à 100 pas : 0,0205 ;
0,144 ; 0,262 ; 0,281 ; 0,310. Graine 59 à 100 pas : 0,00782 ; 0,0143 ; 0,0234 ; 0,0515 ; 0,0775. Une bande plus large tient
des nœuds du corps de la colonie (la spore est à 2,5 cases du bord). **Une colonne** est celle de `piece::traversee` :
`K > 0` exactement quand la colonie traverse.

**« Traverse » et `piece::traversee`.** Au moins un nœud dans chaque mors, contre un chemin de cases occupées d'un bord à
l'autre : aucun désaccord sur 9 960 comparaisons du prototype (les six scénarios de la page, la petite boîte pas à pas) ni sur
les 300 des tests. C'est attendu (la colonie est d'un seul morceau, et un segment qui touche une colonne y a un bout), mais ce
n'est pas un théorème : un nœud posé exactement sur une ligne de la grille, ou déplacé par la fonte, peut faire exception.

**Le solveur plein** (prototype ; une chaîne de poutres, ordre de grandeur) : 100 nœuds, 6 ms et 4 Mo ; 300 nœuds, 0,12 s et
18 Mo ; 1 000 nœuds, 4,2 s et 154 Mo ; 1 500, 14 s et 335 Mo. D'où le plafond de 1 000 nœuds. Une colonie de la petite
boîte le dépasse souvent avant 100 pas (37 états sur 130), et celles de la boîte de la page bien avant de traverser.

**La précision.** Les colonies à la main sont exactes à 4e-15 près. Une colonie simulée est mal conditionnée : son plus petit
mode (voir `systeme`) va de 1,8e-9 à 1e-5 environ (93 colonies) ; la même colonie calculée dans un autre ordre (les bords échangés,
les nœuds à rebours) rend `K` à 2e-7 près au pire (120 graines). **Six à sept chiffres**, pas quinze.

**La raideur d'une colonie qui pousse** (petite boîte, graine 8, `EA` = 22,9 et `EI` = 18,3/49) :

| Pas | 10 à 30 | 40 | 50 | 60 | 70 | 80 | 90 | 100 |
|---|---|---|---|---|---|---|---|---|
| Nœuds, contacts, ponts | 33 à 141 | 174, 2, 2 | 178, 4, 3 | 188, 8, 7 | 195, 10, 9 | 196, 10, 9 | 203, 13, 12 | 213, 14, 13 |
| `K` (µN par case) | 0 | 4,812532e-3 | 4,970316e-3 | 5,034065e-3 | 5,049704e-3 | 5,049704e-3 | 5,542710e-3 | 5,685604e-3 |

Zéro tant que la colonie ne touche pas le bord droit (vers le pas 34), puis **croissante** ; sans les ponts elle reste à
3,389775e-3 (+68 % avec eux à 100 pas). Retirer une poutre n'élève jamais `K` : c'est un théorème. Ajouter un contact, ou
dix pas de croissance, ne l'abaisse pas non plus tant que la fonte ne déplace que des pointes : sur 68 états de 120 graines,
la plus forte baisse est de 6e-9, la précision du calcul.

## Les cas exacts

`cargo run --release --example colonie_raideur` écrit les nombres de la colonie de démonstration (graine 42, 60 pas : 79
segments, 85 nœuds, 90 poutres dont 6 ponts, `K` = 0,0752 µN par case) avec le statut de chacun et, avec un dossier,
`etape-15-raideur.png`. Les neuf colonies à la main des tests (`EA` = 5, `EI` = 3, des coordonnées exactes en `f64`) :

| Colonie | `K` exact |
|---|---|
| Une hyphe droite en quatre segments inégaux, de longueur 4 | `EA/L` = 5/4 |
| Deux hyphes droites en parallèle | `2 EA/L` = 5/2 |
| La chaîne et une branche libre (une console de deux poutres) | 5/4 : la branche ne porte rien |
| Une poutre inclinée 3-4-5 d'un mors à l'autre | `EA c²/L + 12 EI s²/L³` = 1701/3125 |
| Une hyphe en Z (traverses de 1, montant de 2 qui fléchit) | 1260/1399 |
| Une boucle fermée par un pont de 0,25 | 5/2, contre 5/4 sans le contact |
| Une échelle : un segment coupé aux 2/5, un pont vertical qui ne porte rien | 5/2 |
| Un pont qui porte (vertical, de 2, en flexion) | 8459601460/5420380133 ≈ 1,5607, contre 5/4 sans le contact |
| La même échelle, la tige finissant sur la première hyphe | 5/2, sans pont |

Et un cadre de 40 × 25 nœuds (le plafond) : 25 rangées de `EA/39`, soit 125/39, les montants ne portant rien.

## Ce que les tests vérifient

- **`etape_15_cas.rs`** : les neuf colonies (nœuds, poutres, ponts comptés sur le dessin ; `K` exact avec et sans contacts ;
  six valeurs redites ; pour la poutre inclinée, les déplacements, les réactions des deux mors, les six efforts et l'énergie) ;
  la coupe au point le plus proche, le pont (de ce point à la pointe, ou du bout visé), deux contacts dans le désordre, les
  nœuds partagés bit à bit ; la fonte (une pointe à 1/256 et à 1/32, une coupe à 1/256 d'un bout, un segment de 1/256, une
  fonte en cascade, la longueur `COURT` tout juste) ; les mors (colonne ou ligne, les deux sens, le dernier nombre avant la
  colonne suivante, les nœuds hors de la boîte) ; une colonie qui ne traverse pas.
- **`etape_15_limites.rs`** : les refus (chaque donnée, à chaque place), les grandeurs hors de la plage normale, le plafond
  (un cadre de 1 000 nœuds passe, 4 s ; 1 001 sont refusés ; il ne compte que si la colonie traverse), ni fonction interdite
  ni `HashMap`, le noyau ne connaît pas le réseau, cette section.
- **`etape_15_invariants.rs`** (les neuf colonies et deux simulées) : la poussée linéaire en `delta` (de −2,5 à 1e6) ;
  `½ K δ²` et la somme des énergies des poutres ; l'équilibre des réactions des mors ; `EA` et `EI` fois 3 donnent `K` fois 3
  et les mêmes déplacements ; `K` ne change ni quand on tient l'autre bord, ni par le miroir, la transposée, l'ordre des
  segments ou leur sens ; **retirer un pont n'élève jamais `K`**, ajouter un contact ne l'abaisse pas ; deux exécutions
  égales au bit près.
- **`etape_15_colonie.rs`** : les contacts rejoués sont les anastomoses du noyau (65 colonies, 39 780 contacts) et, pour les
  six scénarios de la page et un mur d'une case à un rayon de 1,5, exactement ceux d'une recherche refaite sans index (le plus
  proche, le plus ancien à distance égale, aucun obstacle entre les deux) ; « traverse » s'accorde avec `piece::traversee`
  (300 comparaisons) ; la courbe de `K` ne baisse pas ; l'exemple (chaque ligne du rapport avec son statut, l'image).

## Ce que les tests ne vérifient pas

- Que la raideur d'une colonie ressemble à celle d'un vrai mycélium : rien ne la compare à une mesure.
- Une colonie de plus de 1 000 nœuds ; le temps de calcul ; Windows (rien n'y a tourné avant la CI).
- Les **valeurs exactes** ne viennent que de ce banc : aucune n'est comparée à une publication.
- Le rejeu après un changement du noyau : il **copie** la règle du noyau (`Index`, rayon, délai, obstacle) ; il ne lit pas un
  état, il le refait. Les tests le comparent au noyau sur les scénarios cités seulement.
- Les nombres d'une colonie simulée (`K` de la démonstration, de la courbe) sont ceux que ce code rend : ils figent le
  calcul, ils ne le prouvent pas.
- Le binaire de l'exemple (ses arguments, son code de sortie) : les tests appellent ses fonctions.
- Le cisaillement, une section ou des raideurs qui varient d'une hyphe à l'autre.

## Les limites

- **Un modèle à nous, pas une raideur de matériau.** `K` dépend de `EA` et `EI` donnés (**uniformes**, alors que la paroi
  vieillit : Brinkmann et coll., fig. S4), de la règle de contact (un pont de même raideur, alors qu'une vraie anastomose
  fond les parois), des mors et de la fonte.
- **Euler-Bernoulli pour des poutres trapues** : un segment fait 0,76 à 0,91 case, soit 5 à 6 µm, pour une hyphe de 2,8 µm
  de diamètre (un élancement de 2). Le cisaillement n'est pas compté ; son ordre de grandeur (3 (h/L)² de la flexion pour une
  section rectangulaire tenue aux deux bouts, d'après la relecture de l'étape 14, non refait ici ni calculé pour une paroi
  annulaire) est de plusieurs dizaines de pour cent. **Un arbre plus des ponts** : les boucles ne viennent que de la règle de
  contact.
- **De petites colonies** : au plus 1 000 nœuds (`INCONNUES_MAX`) ; au-delà `TropGrand`. Un gradient conjugué est une suite
  possible, pas cette étape. En deux dimensions, petits déplacements, comportement linéaire.
- **Des grandeurs dans la plage normale de `f64`** : `ea`, `ei` et `delta` normaux, des coordonnées de moins de 1e50, des
  longueurs dont le carré (pour `extraire`) et le cube (pour `pousser`) sont normaux. Un résultat qui déborde ou s'efface est
  **refusé** : `delta` = 1e300 avec `EA` = 1e10 rendrait `NaN`, `delta` = 1e-300 avec `EA` = 1e-100 rendrait zéro. C'est aussi
  le sort d'un réseau fait à la main en deux morceaux, un par mors : sa force est nulle. Avec `delta` = 1, de `EA` = 1e-250
  à 1e300, le calcul est juste à 2e-15 près (mesuré par le prototype ; les tests rejouent les deux bouts de la plage).
- **Six à sept chiffres** pour une colonie simulée (voir « La précision ») ; un réseau dont le plus petit mode tombe sous
  1e-12 est refusé comme mécanisme, même valide.

# Les lectures de la page (étape 16)

**Rien de la simulation ne change.** L'étape donne à la page trois lectures de plus, en lecture seule : le pas de
naissance de chaque segment, l'hyphe qui l'a laissé et la position de chaque pointe vivante. La page pourra en tirer
l'âge d'un segment, son flux (le nombre de segments de son sous-arbre, qu'elle refera depuis l'hyphe de chacun) et ses
pointes ; elle ne les emploie pas encore. L'état, l'écriture canonique, les empreintes et le résultat des 18
exportations d'avant sont ceux de l'étape 15.

## Ce qui est exposé

- **Le noyau** : `Simulation::etat_des_pointes(&self)` rend un itérateur (sans allocation) d'`EtatPointe { id, x, y,
  actif }`, un par apex, dans l'ordre des identifiants (0, 1, 2…, sans trou : aucun apex n'est retiré). Un apex
  anastomosé ou arrêté par un obstacle reste, avec `actif` à faux : il y a `1 + ramifications()` états, et les vivants
  sont les `pointes()`. Ni la direction ni la mémoire de glissement d'un apex ne sortent.
- **La vitrine** : `mycelium_segment_temps(i)` (le pas de naissance, à partir de 1), `mycelium_segment_apex(i)`
  (l'identifiant de l'apex, à partir de 0) et `mycelium_pointe(i, k)` (`k` = 0 pour `x`, 1 pour `y` de la `i`-ième
  pointe **vivante**, dans l'ordre des identifiants ; `mycelium_pointes()` en donne le nombre). Hors de portée (`i` trop
  grand, `k` plus grand que 1) et avant tout démarrage : `NaN`, comme `mycelium_segment`.
- **Les deux entiers sont rendus en `f64`** : 0 est un identifiant d'apex valide, aucun entier ne reste libre pour dire
  « hors de portée », et la règle du module est déjà « hors de portée : `NaN` » (`mycelium_obstacle` rend ses entiers de
  même) ; un entier de moins de 2^53 est exact en `f64`.

## La position d'une pointe

Un apex ne bouge qu'en laissant un segment, qui part de là où il était et au bout duquel il arrive. Sa position est donc
le bout `(x1, y1)` de **son dernier segment** ou, s'il n'en a laissé aucun, **le point où il est né** : la spore (au
centre de sa case) pour l'apex 0 ; pour un autre, le bout du segment que sa mère vient de laisser (ramification), ou la
position de sa mère si elle vient de s'arrêter contre un obstacle (naissance de contact, deux branches au plus). Un apex
bloqué (par le bord, par un mur sous `thigmotropisme 0`, ou faute de substrat) garde sa place ; un apex anastomosé reste
au bout de son dernier segment. Les tests le vérifient à chaque pas, en comparant l'état d'après à celui d'avant, contre
les segments et les compteurs du noyau ; sur leurs parcours, chacun des six cas se présente (sur 6 scénarios et 8
graines, le juge en a compté 92 382 pas avancés, 2 001 anastomoses, 42 190 pas bloqués, 69 arrêts, 4 643 naissances par
ramification et 138 de contact).

## Ce que ça coûte

`mycelium_pointe` parcourt les apex à chaque appel : lire toutes les pointes d'une image coûte le produit du nombre de
pointes par celui des apex. Mesuré sous Node 20 (VM partagée, médiane de 200 lectures de toutes les pointes, `x` et `y`) :
4 µs pour `mur` (graine 1, 350 pas : 15 pointes), 50 à 60 µs pour `libre` (graine 1, 130 pas : 227 pointes sur 342
apex), et 0,3 ms au pire trouvé (`libre`, graine 12, 264 pas : 384 pointes sur 1 020 apex, le plus grand produit des six
scénarios pour les graines 1 à 20 et les pas 1 à 400). Lire les coordonnées de tous les segments, ce que la page fait
déjà, coûte davantage. **Aucun cache** : sans mesure qui le demande, le plus simple.

## Ce que les tests vérifient

- **`etape_16_noyau.rs`** : un seul état au départ, sur la spore ; à chaque pas (6 scénarios, 5 graines, jusqu'à 40 pas
  après le pas de référence) le nombre, l'ordre et les vivants, et la règle de position ci-dessus ; la lecture ne change
  ni l'écriture canonique ni l'empreinte (comparées à une simulation jumelle jamais lue).
- **`etape_16_vitrine.rs`** : les trois lectures contre `Simulation`, au bit près (6 scénarios, 2 graines, 6 moments,
  dont le pas 1100, où plus aucune pointe ne vit) ; `NaN` au premier indice de trop, bien au-delà et pour `k` de 2, jamais
  au dernier indice valide ; une lecture complète à chaque pas ne change pas l'empreinte.
- **`etape_16_bornes.rs`** : les bornes que la page atteint (les pas 200 et 3000 en plus, et `terrain`, graine 6, au pas
  1100 : des pointes vivantes d'identifiant au-delà de 1000, des segments nés après le pas 1023) ; après les lectures de
  chaque moment, l'empreinte et la durée inchangées, même sans pointe vivante ; l'accesseur contre le noyau jusqu'au pas
  3000 (`1 + ramifications()` états, chaque apex au bout de son dernier segment).
- **`etape_16_artefact.rs`** et `tests/etape_16/pointes-wasm.mjs` : le module (les 21 exportations, leurs signatures, `NaN`
  avant tout démarrage), puis, sous Node, les trois lectures égales au bit près à celles du code natif ; et, jamais
  ignoré, que le module a les trois lectures si et seulement si `etape-16` est active.
- Trois tests des étapes d'avant changent, et eux seuls : le compte des 18 exportations (`etape_11_artefact.rs`, non
  compilé avec `etape-16`), les exportations que `app.js` doit employer (`etape_4_artefact.rs` : trois noms réservés à la
  page suivante) et l'accesseur du noyau (`etape_15_limites.rs` : il ne refuse plus que celui qui écrirait).

## Ce que les tests ne vérifient pas

- Que la page emploie ces lectures : elle ne le fait pas encore.
- Les octets du module : ils peuvent changer avec une ligne de `src/*.rs` qui déplace un message de panique compilé dans le
  module (son numéro de ligne y est), et changent avec les fonctionnalités de `default`. Ce qui est figé, c'est le nom, la signature et le résultat de chaque
  exportation ; le module se reconstruit après la dernière modification de `src/` et de `Cargo.toml`.
- Les lectures avant tout démarrage en natif (l'état global est déjà posé par les autres tests) : seul le module neuf le
  montre.
- Windows, un navigateur, et des colonies plus grandes que celles mesurées (lire toutes les pointes y coûte le produit du
  nombre de pointes par celui des apex).
