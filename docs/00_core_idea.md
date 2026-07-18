# Core Idea

`EF Engine` is a tiny `Entropy Folding` pattern:

1. make a bounded attempt
2. detect failure or stuckness honestly
3. vault the failure as evidence
4. retry differently
5. triangulate the exact lock point
6. metabolize the lesson into a constraint
7. route future attempts through the new boundary

That is the whole mechanism.

## Mental Model

- attempts are bounded moves
- vault entries preserve failed evidence
- retry deltas prove what changed
- triangulation isolates the shared lock
- constraints store metabolized negative knowledge
- routing decisions apply that knowledge forward

## Why It Exists

A lot of systems fail in ways that teach them nothing because the failure is:

- hidden
- smoothed over
- repeated without change
- remembered only as prose

This pattern keeps the failed search inspectable and turns it into reusable
future structure.
