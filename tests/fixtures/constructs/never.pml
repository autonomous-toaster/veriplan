/* A never claim that matches every run: pan reports a violation. */
byte x = 0;

active proctype P() {
    x = 1
}

never { skip }
