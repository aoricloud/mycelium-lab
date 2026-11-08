"""Loi d'équilibre exacte de la pointe seule dans un gradient constant (étape T1, chimiotropisme).

Hors du code testé : Python pur, pas de numpy. Les tables COS/SIN sont lues dans src/croissance.rs (ce sont des
données, pas de la logique) et recoupées avec math.sin / math.cos.

Le noyau d'un pas de la pointe, d'après la spécification (section « Le virage ») :
    d1 = d + delta                 delta uniforme sur -A..=A (2A+1 valeurs)
    v  = P * sinus(d1)             P = force * s ; sinus = sin(angle du gradient - d1)
    d2 = d1 + floor(v) + Bernoulli(v - floor(v))      (mod 360)
Le gradient est (g, 0), donc angle du gradient = 0 et sinus(d1) = -SIN[d1] (même expression que dans le code :
(COS[d]*gy - SIN[d]*gx)/n avec gx = n, gy = 0).

Sortie : pour chaque (A, P) la moyenne E[cos(theta - theta_g)] = somme pi[d] * COS[d] à l'équilibre, obtenue en
résolvant pi (T - I) = 0 (élimination de Gauss avec pivot), vérifiée par le résidu pi T - pi, et la variance
asymptotique de la moyenne d'une longue trajectoire (équation de Poisson (I - T) g = f - pi f).
"""
import math
import re
import sys

N = 360


def tables(chemin):
    texte = open(chemin, encoding="utf-8").read()
    def lire(nom):
        m = re.search(r"pub const " + nom + r": \[f64; NB_DIRECTIONS\] = \[(.*?)\];", texte, re.S)
        valeurs = []
        for ligne in m.group(1).splitlines():
            ligne = ligne.strip().rstrip(",")
            if not ligne:
                continue
            if "FRAC_1_SQRT_2" in ligne:  # std::f64::consts::FRAC_1_SQRT_2 : 0.7071067811865476
                valeurs.append(-0.7071067811865476 if ligne.startswith("-") else 0.7071067811865476)
            else:
                valeurs.append(float(ligne))
        assert len(valeurs) == N, (nom, len(valeurs))
        return valeurs
    return lire("COS"), lire("SIN")


COS, SIN = tables(sys.argv[1] if len(sys.argv) > 1 else "src/croissance.rs")
# Recoupement avec la bibliothèque mathématique de Python.
ecart = max(max(abs(COS[i] - math.cos(math.radians(i))), abs(SIN[i] - math.sin(math.radians(i)))) for i in range(N))
assert ecart < 1e-15, ecart


def noyau(A, P):
    """Lignes éparses du noyau : T[d] = liste de (d2, probabilité)."""
    T = []
    for d in range(N):
        ligne = {}
        for delta in range(-A, A + 1):
            d1 = (d + delta) % N
            v = P * (-SIN[d1])
            bas = math.floor(v)
            frac = v - bas
            for cible, p in (((d1 + bas) % N, 1.0 - frac), ((d1 + bas + 1) % N, frac)):
                if p > 0.0:
                    ligne[cible] = ligne.get(cible, 0.0) + p / (2 * A + 1)
        T.append(sorted(ligne.items()))
    return T


def resoudre(M, b):
    """Élimination de Gauss avec pivot partiel : M x = b (M dense n x n, copie)."""
    n = len(b)
    a = [M[i][:] + [b[i]] for i in range(n)]
    for k in range(n):
        p = max(range(k, n), key=lambda i: abs(a[i][k]))
        a[k], a[p] = a[p], a[k]
        pivot = a[k][k]
        assert abs(pivot) > 1e-14, "matrice singulière"
        ligne_k = a[k]
        for i in range(k + 1, n):
            f = a[i][k] / pivot
            if f != 0.0:
                ligne_i = a[i]
                a[i] = ligne_i[:k] + [x - f * y for x, y in zip(ligne_i[k:], ligne_k[k:])]
    x = [0.0] * n
    for i in range(n - 1, -1, -1):
        s = a[i][n] - sum(a[i][j] * x[j] for j in range(i + 1, n))
        x[i] = s / a[i][i]
    return x


def stationnaire(T):
    # pi (T - I) = 0  <=>  (T^T - I) pi^T = 0, une équation remplacée par somme(pi) = 1.
    M = [[0.0] * N for _ in range(N)]
    for d in range(N):
        for cible, p in T[d]:
            M[cible][d] += p
    for i in range(N):
        M[i][i] -= 1.0
    M[N - 1] = [1.0] * N
    b = [0.0] * N
    b[N - 1] = 1.0
    pi = resoudre(M, b)
    # Résidu.
    nouveau = [0.0] * N
    for d in range(N):
        for cible, p in T[d]:
            nouveau[cible] += pi[d] * p
    residu = max(abs(nouveau[i] - pi[i]) for i in range(N))
    return pi, residu


def poisson(T, pi, f):
    """g tel que (I - T) g = f - pi.f, normalisé par pi.g = 0 ; rend (g, pi.f)."""
    moyenne = sum(pi[d] * f[d] for d in range(N))
    ft = [f[d] - moyenne for d in range(N)]
    M = [[0.0] * N for _ in range(N)]
    for d in range(N):
        M[d][d] += 1.0
        for cible, p in T[d]:
            M[d][cible] -= p
        for j in range(N):
            M[d][j] += pi[j]  # + 1 pi
    g = resoudre(M, ft)
    return g, ft


def analyse(A, P, axe="x"):
    T = noyau(A, P)
    pi, residu = stationnaire(T)
    f = COS[:]  # cos(theta - theta_g) avec theta_g = 0
    moyenne = sum(pi[d] * f[d] for d in range(N))
    g, ft = poisson(T, pi, f)
    # variance asymptotique de la moyenne : 2 <ft, g>_pi - <ft, ft>_pi
    var_stat = sum(pi[d] * ft[d] * ft[d] for d in range(N))
    sigma2 = 2.0 * sum(pi[d] * ft[d] * g[d] for d in range(N)) - var_stat
    return moyenne, residu, var_stat, sigma2


if __name__ == "__main__":
    A = int(sys.argv[2]) if len(sys.argv) > 2 else 20
    produits = [float(x) for x in sys.argv[3:]] or [0.0, 0.25, 0.5, 1.0, 2.0, 4.0]
    print("A  P(=force*s)  E[cos]  residu  var_stationnaire  sigma2_asymptotique")
    for P in produits:
        m, r, vs, s2 = analyse(A, P)
        print(f"{A} {P} {m:.8f} {r:.1e} {vs:.6f} {s2:.4f}", flush=True)
