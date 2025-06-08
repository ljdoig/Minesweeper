# Minesweeper
This project uses [bevy](https://bevyengine.org/), a Rust game engine, to recreate the classic puzzle game Minesweeper.
The main purpose was to create an algorithm for playing Minesweeper, described below.
To see the algorithm in action check out the [github-pages deployment](https://ljdoig.github.io/Minesweeper/) of this repo.
I've slowed it down so you can see it in action, but on average it finishes games in ~50ms and wins just over 50% of the time, which is about as good as it gets for Minseweeper.

## Example simulation
```
Game 500 finished in 0.04s
Record: 257-243-0 on Hard (51.40% win rate, 88.09% bombs cleared)
50ms per game, 25.03s in total, longest game took 0.61s
```
## Intro

If you're unfamiliar with Minesweeper, it's a game of boolean logic in which you are given a grid of tiles, with some labelled with integers between
1 and 8, and some covered up. The integers tell you how many of the surrounding tiles contain bombs.
Clicking on a bomb-tile detonates it, while right-clicking a bomb-tile flags it as unsafe.
Clicking a non-bomb tile reveals another integer, showing how many of its neighbours are bombs, allowing you to deduce more safe or unsafe tiles.
The game ends when you detonate a bomb, losing, or you flag all bombs, winning.

The game playing agent comes in two parts:
1. A deterministic process to check if any tiles are guaranteed to be safe or unsafe using deductions from known subsets/supersets of tiles
2. A probabilistic process to identify the tile that is most likely to be safe, which is then uncovered


## Deterministic Logic
This process involves storing sets of tiles along with bounds on the number of bombs they may contain.
The goal is to find a nonempty set of $n$ tiles with at most 0 bombs (all safe) or at least $n$ bombs (all bombs).
The following deductions are made recursively:
1. Size constraints: every set of $n$ tiles contains between $0$ and $n$ bombs inclusive.
2. Subsets: if a set of tiles contains at most $k$ bombs, every subset of these tiles contains at most $k$ bombs.
3. Recursive #1: For every set of tiles $t$ that contains at MOST $k$ bombs and for every subset $s$ of $t$
    that contains at LEAST $m$ bombs, the remaining tiles $t - s$ contain at MOST $k - m$ bombs.
4. Recursive #2: For every set of tiles $t$ that contains at LEAST $k$ bombs and for every subset $s$ of $t$
    that contains at MOST $m$ bombs, the remaining tiles $t - s$ contain at LEAST $k - m$ bombs.

Rule 4 is equivalent to rule 3, and is just a relabelling of bomb as 'not bomb' and safe as 'not safe'.
To apply these rules to minesweeper we do the following:

1. For every uncovered tile displaying the number $k$, the set of adjacent tiles contains exactly $k$ bombs.
2. Recursively apply the above rules to all sets we have information about.
3. If a set of $n$ tiles contains at least $n$ bombs, flag them all.
4. If a set of tiles contains at most $0$ bombs, uncover them all.

This is roughly how a human plays Minesweeper.

## Probabilistic Logic
Unfortunately this is not enough, and nothing ever will be. Not all games of Minesweeper can be won with the information available.
We can do better on average, however, by generating all possible assignments of bombs that satisfy the given constraints, then finding the tile that is least likely to be a bomb based on these scenarios.

### High level overview
This algorithm begins by considering only the covered tiles that we have information about, i.e. the covered boundary of tiles adjacent to numbered unconvered tiles.
The game gives us a set of constraints, tellings us for some subsets of the boundary how many bombs are present.
At a high level, we break the boundary into small sections, then in each section, generate all possible assigments and filter them
down to those that satisfy overlapping constraints. Once we have these, we merge neighbouring sections together, mergesort style,
and check any constraints that overlap both sections. We do this until we have merged all sections together, at which point we have
the set of assignments that satisfy all constraints. This approach lets us eliminate candidates early in each section, avoiding combinatorial explosion.
Once we have all these scenarios, we count how many times each bit is set to work out the safest tile, weighting for the probability of each scenario.
This is all done using bitwise operations and integer comparison, making it very quick.

### Encoding constraints and generating legal bomb assignments
We encode each constraint as an integer and a bit array.
The integer is how many bombs are present, while the bit array tells us which tiles the constraint applies to.
This implementation uses a u128 integer for the bit array in Rust, which is enough for realistic scenarios on a standard Minesweeper board.
For example, $(010...001001, 2)$ means that there are 2 bombs present in the tiles with zero-based indexes 0, 3 and 126.

Once we've encoded all constraints, we partition the boundary into sections of some chosen size $n$.
We then generate all integers between $0$ and $2^n - 1$ as u128 values, and bit shift them so they line up with the corresponding section of the boundary.
For example if we choose $n = 8$ and are considering tiles 32 to 39 of the boundary, we will create a list of values between $0$ and $255$, then shift them all 32 bits to the left. These values, together with a single u128 mask with bits 32 to 39 set, give us the set of candidate 'assignments' of this section of the boundary. For a given assigment, if a bit is set at index $i$ it means a bomb is present at tile $i$, and for the remaining tiles in the mask there is no bomb present.

We then test each of these against all our constraints.
Lets say we have some candidate assignment $c$, of tiles with a mask $m$.
Lets say we are testing this against some constraint that says there are $n$ bombs in the set of tiles $t$.
We check:
1. If $c$ assigns every tile in $t$, then $c$ has $n$ bits set in t.
2. If $c$ leaves $k$ tiles in $t$ unassigned, then $c$ has between $n - k$ and $n$ (inclusive) bits set in $t$.

These can be written as bitwise operations. If $|x|$ tells us how many bits are set in $x$, then we check:
1. $(c\land t)=t\implies|c\land t|=n$
2. $n-|\lnot m\land t|\leq |c\land t|\leq n$

$\lnot m \land t$ are the $k$ tiles in $t$ that $c$ does not assign. Rule 2 lets us check whether this constraint is guaranteed to be violated at some point, since there is no
way that the remaining tiles in $t$ can be assigned to satisfy the constraint.

I won't describe the remaining bitwise operations in detail. Once we have independently generated candidates for each section of
the boundary, we merge adjacent sections together in pairs, using bitwise 'or'. For each pair, we then recheck constraints that overlap both of
the original sections. All other constraints either fully overlap an original section, and so have already been verified, or do not overlap either
original section. This means that minimal work is required on each merge and we get rid of candidates as soon as possible.

### Assigning probabilities to legal scenarios
There is one more complexity after generating all possible boundary scenarios, which is that not all boundary scenarios are equally likely. This would be true if there were no
non-boundary tiles, however in general we need some combinatorics to tells us how many ways we can assign non-boundary bombs
to non-boundary tiles, since it's the bombs that are uniformly distributed not the boundary scenarios.
To be more specific if we have a valid scenario that leaves $b$ bombs to be assigned to $n$ non-boundary tiles,
then this scenario should be given the weight $n \choose b$. Note that we don't have to weight for the bombs we have assigned on the boundary, since we generate *all* scenarios with this many bombs on the boundary. However, since this number is often too big to store, we calculate the minimum number of non-boundary bombs $b'$ across all scenarios, and then divide all weights by $n \choose b'$ giving smaller but still proportional weights. For ease, I'll write $\prod(a, b)$ to mean $\prod_{i=a}^{b} i$. Each scenarios probability weight is:
```math
{{n}\choose{b}}/{{n}\choose{b'}}
=
\frac{
    \prod(n-b+1, n)
    }{
    \prod(1, b)
}
/
\frac{
    \prod(n-b'+1, n)
    }{
    \prod(1, b')
}
=
\frac{
    \prod(n-b+1, n-b')
    }{
    \prod(b'+1, b)
}
```
Now we have $b - b'$ terms in the numerator rather than $n - b$ terms, which is much smaller.

Finally, it can be even safer to pick a random non-boundary tile. Even though we have no specific information about
each of these tiles, we do know how many non-boundary tiles there are and how many bombs there are remaining. If this proportion of non-boundary bomb squares is better than our best shot on the boundary, we can instead just pick one of them.

### Aside: boundary ordering

Ideally you want tiles that have common constraints to be nearby each other (in terms of boundary indexes) so that
these constraints are considered as early as possible, and so there are fewer
constraints considered when merging. The approach implemented is to pick the
two tiles that are the furthest apart, then partition all tiles based on which of the
two is closer. The same method is then recursively applied to each half, then they are
concatenated. There is potentially a better approach based on the constraints themselves.
