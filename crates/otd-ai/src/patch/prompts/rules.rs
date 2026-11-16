/// Appended to the system prompt when the editor has removal switched off.
///
/// The prompt above teaches `delete` unconditionally, because most of the time
/// it is the right answer — "make it simpler" is a delete list. But a delete
/// is the one thing a plan does that the user cannot see coming, so the editor
/// makes it opt-in and this is what it says when it is off. Belt and braces:
/// the caller drops any `delete` that comes back anyway, since a rule in a
/// prompt is a request rather than a guarantee.
pub const NO_DELETE_RULE: &str = "\n\nREMOVAL IS OFF FOR THIS REQUEST\n\
     Do not use `delete`. Every operator now in the network stays. Add to it, \
     rewire it, retune it with `set` — but remove nothing, and do not work \
     around this by emptying an operator out or wiring it to nothing. If what \
     was asked for can only be done by removing something, do the part that \
     can be done and say in `notes` what would have to go and that removal is \
     switched off.";

/// Told to the model when the user has locked the request to a selection.
///
/// Paired with [`gate_scope`], which is what makes it true. The prompt is how
/// the model avoids wasting the request on operators it is not allowed to
/// touch; the gate is why "only change these" is a guarantee rather than a
/// hope.
pub const SCOPE_RULE: &str = "\n\nONLY THE OPERATORS MARKED [IN SCOPE] MAY CHANGE\n\
     The user selected part of the network and asked for that part to be \
     worked on. Operators marked [IN SCOPE] are yours: retune them with \
     `set`, rewire their inputs, remove them. Every operator marked [LOCKED] \
     stays exactly as it is — do not `set` it, do not `delete` it, and do not \
     wire anything into its inputs.\n\
     Reading FROM a locked operator is fine and is usually how this works: \
     name it as the `from` of a connection and take its output. What you may \
     not do is change what goes INTO it.\n\
     You may still create operators, and anything you create is in scope by \
     definition — wire new ones to each other and into the selected ones \
     freely. If the request cannot be done without changing a locked \
     operator, do the part that can be done and say in `notes` which operator \
     would have to change and that the selection is locked.";
