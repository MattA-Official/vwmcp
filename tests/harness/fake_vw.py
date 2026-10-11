"""Plays Vectorworks: runs the menu-command one-liner every 50 ms, in one
long-lived interpreter, exactly as VW's script runner would on each trigger."""
import os, sys, time
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), 'fake_vs'))
script = sys.argv[1]
deadline = time.time() + float(sys.argv[2])
while time.time() < deadline:
    try:
        exec(script, {'__name__': '__main__'})
    except FileNotFoundError:
        pass  # server has not deployed entry.py yet
    time.sleep(0.05)
