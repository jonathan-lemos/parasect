<h1 align="center">parasect</h1>

<p align="center">
    <img src="https://static.wikia.nocookie.net/pokemon/images/8/80/047Parasect.png/revision/latest" alt="Parasect" height="150px" width="150px"/>
    <br>
    A <b>para</b>llel bi<b>sect</b>ion tool
    <img src="docs/demo.gif" alt="demo run"/>
</p>

## Installation

Go to the [releases](https://github.com/jonathan-lemos/parasect/releases) page and download the latest binary for your
OS/CPU architecture, then put it in a directory in your `$PATH`.

## Usage

Example usage is as follows

```
parasect --low=50 --high=500 -- YOUR_BINARY_HERE --flag1 --flag2 pos1 '$X'
```

This will parasect `YOUR_BINARY_HERE` with the given arguments and `'$X'` replaced with a number.
It will return the first number within 50 and 500 inclusive that, given to `YOUR_BINARY_HERE`, returns a value != 0.

### Optional arguments

| Argument                  | Description                                                                                                                             |
|---------------------------|-----------------------------------------------------------------------------------------------------------------------------------------|
| `--max-parallelism=N`     | The maximum amount of threads to start. Must be >0. By default, this is the number of logical CPU's on the system.                      |
| `--no-tty`                | Disable the fancy terminal interface and output a stream of logs instead. This will automatically be turned on if `stdout` is not a TTY |
| `--substitution-string=S` | Put the number in the given string instead of `$X`.                                                                                     |