# Introduction to SkyHetu

SkyHetu is a general-purpose programming language designed to solve the "Hidden State" problem.

In modern software engineering, complexity arises from implicit state changes, invisible side effects, and loss of history. SkyHetu makes these elements explicit.

## Key Principles

1.  **Immutability by Default**: Variables defined with `let` cannot change. This forces you to be intentional about what *needs* to change.
2.  **Explicit State**: Mutable variables must be declared with `state`.
3.  **Visible Transitions**: You don't "assign" to state; you "transition" it using `->`. This syntax makes mutation visually distinct from equality or binding.
4.  **Causality Tracking**: The runtime remembers the history of every state variable.

## A "Hello World" Example

```sky
// Simple print
print("Hello, SkyHetu!")

// A stateful counter
state n = 0

// A transition
n -> n + 1

// Querying history
print(why(n))
```

## Comparisons

| Feature | Python/JS | SkyHetu |
|---------|-----------|---------|
| Assignment | `x = 5` | `let x = 5` (immutable) |
| Mutation | `x = 6` | `x -> 6` (tracked) |
| Debugging | Print / Debugger | `why(x)` / `causal_graph` |
| OOP | Classes + Inheritance | Classes + Composition |

## Next Steps

- Learn about [Installation](../guide/installation.md)
- Explore the [Causality Engine](../guide/causality.md)
- Understand [Modules](../guide/modules.md)
