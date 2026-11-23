/* Control: the same model passing 8 to an assert of 7, which any correct binding refutes. */
proctype P(byte v) { assert(v == 7) }
active proctype Main() { run P(8) }
