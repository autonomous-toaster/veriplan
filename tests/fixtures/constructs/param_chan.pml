/* A channel passed as a parameter: the callee must send on the caller's channel. */
chan c = [1] of { byte };
byte got = 0;

proctype S(chan out) { out ! 5 }
active proctype Main() { run S(c); c ? got; assert(got == 5) }
