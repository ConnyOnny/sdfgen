# Signed Distance Field Generator

## What's a Signed Distance Field?

Signed Distance Field Rendering is a technique for efficiently rendering 1-bit vector textures on a GPU.

For the original science paper see here: http://www.valvesoftware.com/publications/2007/SIGGRAPH2007_AlphaTestedMagnification.pdf

For a video explanation see here: https://www.youtube.com/watch?v=CGZRHJvJYIg

## What does this program do?

For applying the above mentioned rendering technique you need special texture images. This program takes a large rendered image of your binary vector graphics (e.g. 4096x4096 px) and generates a usable signed distance field texture (e.g. 128x128 px) out of it.

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
