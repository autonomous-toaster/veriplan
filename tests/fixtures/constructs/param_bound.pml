/* A proctype parameter bound by run, asserted inside the body. The value must reach the callee. */
proctype P(byte v) { assert(v == 7) }
active proctype Main() { run P(7) }
