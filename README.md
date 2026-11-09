# Mycelium Lab

A free simulator of mycelium growth, in Rust, built for research: a virtual lab. It grows a network of hyphae (tip
extension, branching, anastomosis, nutrient transport), measures it (biomass, density, topology, fractal dimension) and
compares scenarios.

**▶ Run it in your browser: https://aoricloud.github.io/mycelium-lab/** (six scenarios, WebAssembly, nothing loaded
from outside the page).

> [!NOTE]
> **This repository is written by AI agents, under [Lydite](https://github.com/aoricloud/lydite).** A human writes the
> direction and keeps the settings; the agents write the code, and Lydite replays the proof before any pull request.
> Recent pull requests carry a receipt (cost, agents, retries) and the line `Reviewed and replayed on commit …`; GitHub
> only merges a pull request whose head is that commit, once the CI is green on Linux and Windows. See
> [#51](https://github.com/aoricloud/mycelium-lab/pull/51), or
> [all merged pull requests](https://github.com/aoricloud/mycelium-lab/pulls?q=is%3Apr+is%3Amerged).
> As of 9 November 2026: 35 pull requests by Lydite merged in three days; the 17 that carry a costed receipt add up
> to $4.91 for the agents (estimated; they run on a low-cost model, DeepSeek; the earlier pull requests had no receipt,
> and the session that writes the plans is not counted).

## Where it stands

The first version (growth, the coupling of the colony and its substrate, a published figure redone in numbers, this web
page) is merged, and so are the tropisms after it: a terrain with walls, growth toward a nutrient patch (chemotropism),
growth along a wall (thigmotropism), each with and without the effect, all six scenarios on the page. Measuring a grown
piece is merged too: a density map, a threshold, the food balance, the largest connected piece and the crossing from one
edge to the other (`cargo run --release --example mesure_piece`). The roadmap's next step is the mould: shapes for the
colony to fill, the maze among them. [MODELE.md](MODELE.md) says, for each model, its equations, its reference and its
limits: this bench is 2D, at the scale of the hypha, and does not predict the strength of a real material.

## What holds it

- Rust with no dependency; the same scenario and the same seed give the same state, on Linux and on Windows.
- The proof is the simulated state and numbers, and a published figure redone in numbers; images are for the eye.
- The page shows the SHA-256 of the state: the same seed gives the same 64 characters in the browser and natively.
- The rules of the repository are in [AGENTS.md](AGENTS.md).

## Try it

```sh
# the full proof: tests, lint and format
cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check

# a one-line scenario
echo 'boite 64 48' > boite.txt
cargo run -- lancer boite.txt --graine 1 --sortie sortie
```

The last command writes `empreinte.txt` and `apercu.png` in `sortie/`. A scenario's address can be shared:
`https://aoricloud.github.io/mycelium-lab/#scenario=mur&graine=31`.

The rest of this repository (the model notes, the rules, the roadmap and the page) is in French.

## License

MIT, see [LICENSE](LICENSE).
