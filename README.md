# OpenGL Rust shader demo template

This is a small template project that uses glutin to create an opengl context and window, and includes an example shader.

## Building and running

    cd shader_demo_example
    cargo run

## How to use this

The exmaple program compiles and links a shader that fills the screen.
Variables:

    pxcoord // the screen coordinates
    time // time in seconds since the program started

pxcoord is calculated from some other uniform variables, 0,0 is the lower left corner, the upper left corner is 1920,1080

simply edit the shader and rerun

you might want to replace the shader string literals with included files with (for example): `include_str!("myshader.frag")`
so that your text editor of choice has a chance of doing syntax highlighting correctly

## Licence

You are free to use this code in your project and to modify as you need.

There is no need for any attribution or copyright messages when you do so.
