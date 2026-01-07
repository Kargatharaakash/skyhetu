# Modules System

SkyHetu supports organizing code into reusable modules using `import` and `export` statements.

## Exporting

Any top-level declaration can be exported using the `export` keyword.

```sky
// math_utils.skyh

export fn add(a, b) {
    return a + b
}

export let PI = 3.14159

export class Vector {
    init(x, y) {
        this.x = x
        this.y = y
    }
}
```

State variables can also be exported, but remember that **causality is local to the runtime**.

```sky
export state global_counter = 0
```

## Importing

You import symbols using the `{ ... }` syntax.

```sky
// main.skyh
import { add, PI, Vector } from "math_utils"

print(add(2, 5))
print(PI)
```

### Path Resolution

- **Relative Paths:** Imports are resolved relative to the current file.
- **Extension:** The `.skyh` extension is optional in the import string.
- **Isolation:** Each module is compiled in its own scope, but in the current version (v0.2.0), they share the same global heap for simplicity.

## Best Practices

1.  **One Module per Logical Unit:** Group related functions (e.g., `math.skyh`, `network.skyh`).
2.  **Explicit APIs:** Only export what is necessary. Keep internal helpers private.
3.  **No Circular Dependencies:** While the runtime usually handles them, it is best design to avoid cycles in your dependency graph.
