# Signed Distance Field Generator

## What's a Signed Distance Field?

Signed Distance Field Rendering is a technique for efficiently rendering 1-bit vector textures on a GPU.

For the original science paper see here: http://www.valvesoftware.com/publications/2007/SIGGRAPH2007_AlphaTestedMagnification.pdf

For a video explanation see here: https://www.youtube.com/watch?v=CGZRHJvJYIg

## What does this program do?

For applying the above mentioned rendering technique you need special texture images. This program takes a large rendered image of your binary vector graphics (e.g. 8192x8192 px) and generates a usable signed distance field texture (e.g. 128x128 px) out of it.

### Building

You need Rust. The dependencies (image and getopts) are on crates.io so they will be automatically fetched and compiled when you run

	cargo build --release

### Usage

	$ ./sdfgen --help
	Usage: ./sdfgen [options] inputimage.png outputimage.png

	Options:
		-h --help           print help
		-v --verbose        show what the program is doing
		-s --size OUTPUT_SIZE
		                    size of the output signed distance field image, must
		                    be a power of 2. Defaults to input size / 4
		--maxdst SATURATION_DISTANCE
		                    saturation distance (i.e. 'most far away meaningful
		                    distance') in half pixels of the input image. Defaults
		                    to input size / 4
		--save-mipmaps BASENAME
		                    save the mipmaps used for accelerated calculation to
		                    BASENAMEi.png, where 'i' is the mipmap level

### Example

Get yourself some vector graphic. This might be letters of a font or it might be something else. I use this one: https://openclipart.org/detail/214643/black-cat-blackandwhite

Render the image really large. I used 6000px height here. Then make the background white instead of transparent (that's important!) and added a white border s.t. the image resolution was a power of two (8192x8192 px in my case). This will look something like this (but way larger):

![input image: a cat](http://cberhard.de/github/sdfgen/cat256.png)

Then we use sdfgen:

	./sdfgen --size 128 /path/to/source/image.png /output/file/name.png

This is the result:

![cat sdf](http://cberhard.de/github/sdfgen/catsdf.png)

While this is not pretty, it is pretty useful. You can use this to render the input image very sharply at any resolution. This is rendered at 4096x4096 px out of the 128x128 px signed distance field texture we generated earlier:

![eyes rendered with the sdf](http://cberhard.de/github/sdfgen/eyessdf.png)

With normal (bilinear) filtering it would have looked like this:

![eyes magnified with bilinear interpolation](http://cberhard.de/github/sdfgen/eyes.png)

But this is not all you can do with an SDF texture. Your pixel shader can also map the different distances to different colors. You can simulate this outside OpenGL with [Gimp](http://en.wikipedia.org/wiki/GIMP)s [Gradient Map](http://docs.gimp.org/en/plug-in-gradmap.html) tool (Colors -> Map -> To Gradient).

![artistic shader using SDF](http://cberhard.de/github/sdfgen/catsdfarts.jpg)
