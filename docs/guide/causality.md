# The Causality Engine

SkyHetu is not just a language; it is a **time machine**.

In traditional programming, when a variable changes, the old value is lost forever. In SkyHetu, every mutation is an event in time. We call this the **Causality Log**.

## The Philosophy

> "To understand the bug, you must understand the history."

SkyHetu treats **Time** as a first-class citizen.
- **Micro-Time:** Every state transition (`->`) increments the logical clock.
- **Explicit History:** You can query the past at any moment.

## The Causal Graph ("The Detective Board")

The most powerful feature of SkyHetu is the ability to visualize the "thought process" of your program.

When you run `causal_graph("my_var", "dot")`, SkyHetu generates a graph connecting every state the variable has ever held.

### Example

```sky
state counter = 0
counter -> counter + 1  // t=1
counter -> counter + 1  // t=2
counter -> counter + 1  // t=3
```

### Visualization

```mermaid
graph LR
    s0[0]
    s1[1]
    s2[2]
    s3[3]
    
    s0 -- "t=1" --> s1
    s1 -- "t=2" --> s2
    s2 -- "t=3" --> s3
    
    style s0 fill:#f9f9f9,stroke:#333,stroke-width:2px
    style s1 fill:#e1f5fe,stroke:#03a9f4,stroke-width:2px
    style s2 fill:#e1f5fe,stroke:#03a9f4,stroke-width:2px
    style s3 fill:#ffecb3,stroke:#ffc107,stroke-width:4px
```

- **Nodes:** Represent the value at a specific point in time.
- **Edges:** Represent the *causal link*—the transition event.

## Introspection Tools

### `why(variable)`

Returns a human-readable history of the variable.

```sky
print(why(counter))
// Output:
// Causality chain for 'counter':
//   1. [t=1] 0 -> 1
//   2. [t=2] 1 -> 2
```

### `transitions(variable)`

Returns the number of times a variable has mutated. Useful for asserting stability.

```sky
assert(transitions(config) == 0, "Config should differ change!")
```

### `snapshot()`

Returns the current **Logical Time**. This is useful for synchronizing events across multiple variables.

```sky
let start_time = snapshot()
// ... do work ...
let end_time = snapshot()
```

## Exporting Data

SkyHetu is designed to integrate with external tools.

- **DOT Format:** For Graphviz and visualizers.
- **JSON Format:** For custom web debuggers or data analysis.

```sky
// Export to JSON
let history_json = causal_graph("counter", "json")
// {"variable":"counter","events":[{"id":0,"timestamp":1,"old":"0","new":"1"}...]}
```
