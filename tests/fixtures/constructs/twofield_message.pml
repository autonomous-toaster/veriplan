/* A multi-field message, arity correct on both sides. pan decides it: 0, the assert holds.

   The engine refuses by name - "a receive that binds more than one variable is not supported" - so it
   gives no verdict. Recorded with pan's answer so the row asserts when the receive side lands; the send
   side parsing was fixed in v0.5.15 and the refusal is now a property of the receive only. */
chan c = [2] of { byte, byte };
byte a = 0;
byte b = 0;

active proctype P() {
	c ! 1, 7;
	c ? a, b;
	assert(b == 7)
}
