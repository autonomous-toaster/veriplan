/* A parameterised active proctype. pan starts an active process with every parameter zero, so this asserts
   a fact about the binding rather than about the body: n == 0 by construction. */
active proctype P(byte n) { assert(n == 0) }
