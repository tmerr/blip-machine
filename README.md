# blip-machine

A little machine that's pretty good at making blip noises. It does this by taking in code and outputting an 
8000Hz PCM stream. A minimal example would be `echo "sin 261.6 1" | cargo run | aplay` which plays a middle C
note for a second. Of course it gets more interesting than that. There are loops:

```
lbl A
sin 261.6 0.5
sin 329.6 0.5
sin 392.0 0.5
sin 523.2 0.5
pjump A 1
```

Run that puppy with `cat program.txt | cargo run | aplay` and it will play through each node of a C major chord. To play them all at once:

```
pfork C4 1
pfork E4 1
pfork G4 1
pjump C5 1

lbl C4
sin 261.6 2
pjump End 1

lbl E4
sin 329.6 2
pjump End 1

lbl G4
sin 392.0 2
pjump End 1

lbl C5
sin 523.2 2
pjump End 1

lbl End
```

You can fork/jump probabilistically by changing the second argument from 1 to something lower. In fact that's all there is to it. The full list of instructions is:

```
lbl x
sin freq duration
pjump x probability
pfork x probability
```
