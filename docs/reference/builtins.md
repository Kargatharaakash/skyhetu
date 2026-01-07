# Built-in Functions Reference

SkyHetu comes with a comprehensive standard library for math, I/O, type checking, and causality introspection.

## I/O Functions

### `print(arg1, arg2, ...)`
Prints values to standard output, separated by spaces.
- **Arguments:** Variadic, any type.
- **Returns:** `nil`.

## Type & Conversion

### `type(value)`
Returns the type of the value as a string.
- **Example:** `type(10)` -> `"number"`, `type("hi")` -> `"string"`.

### `len(value)`
Returns the length of a string or array (future).
- **Arguments:** `String`. (Arrays coming in v0.3).
- **Example:** `len("hello")` -> `5`.

### `str(value)`
Converts any value to its string representation.

### `num(value)`
Converts a string to a number. Returns `nil` if conversion fails (v0.2 behavior matches Rust `parse().ok()`).

## Math

All math functions operate on floating-point numbers.

- `abs(n)`: Absolute value.
- `floor(n)`: Largest integer less than or equal to `n`.
- `ceil(n)`: Smallest integer greater than or equal to `n`.
- `round(n)`: Nearest integer.
- `min(a, b)`: Smaller of two numbers.
- `max(a, b)`: Larger of two numbers.

### `range(start, end)` / `range(end)`
Creates a simplified iterator for `for` loops.
- `range(5)` -> `0, 1, 2, 3, 4`
- `range(2, 5)` -> `2, 3, 4`

## Causality & Time

### `why(variable)`
Returns the formatted causality log for a given state variable.
- **Arguments:** State variable (runtime reference).
- **Returns:** `String` (multi-line).

### `causal_graph(variable_name, format)`
Exports the causality history.
- **variable_name:** `String` (name of the variable).
- **format:** `"dot"` (Graphviz) or `"json"`.
- **Returns:** `String` containing the graph data.

### `transitions(variable_name)`
Returns the count of state transitions for a variable.
- **variable_name:** `String`.
- **Returns:** `Number`.

### `snapshot()`
Returns the current Logical Clock timestamp.
- **Returns:** `Number` (integer).

## Utility

### `time()`
Returns the current system time (in seconds/ticks, implementation defined).

### `assert(condition, message?)`
Aborts execution if `condition` is false.
- **message:** Optional string.
