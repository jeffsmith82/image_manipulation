# Image manipulation   

This is intended as a http endpoint you can send image data and ask for a response that is a transformed version of your images. 

## usage

You should send your image data as the body.

You should then use headers to tell the server what image is and what transform you want to be performed

To tell the server what image format you have sent you should use the Content-Type header liek so

> Content-type : image/bmp

The other acceptable mime types are

* image/gif
* image/jpeg
* image/png

If the output image format has a compression level you can tell the server to set it between 1 and 100 like this. The default will be 80 otherwise.

> X-Compress: 80

To tell the server what image type you want back set the Accept header to the image type. We will default to jpeg if this is not set.

> Accept: image/jpeg

* image/gif
* image/jpeg
* image/png

If you want the image resized you can send the following header with the dimension with width and height in pixels and an x in the middle. The image may not be the exact size on output as we try to keep the aspect ratio.

> X-Size: 800x600

If you want the width and height to be exact and dont mind the aspect ratio of the image being distorted then set this header to true.  

> X-ignore-Aspect-Ratio: true

If you want the image cropped then send the following header with the dimension of teh stating pixel and the width and height you want cropped out. This will start at 10 pixels right and 10 pixels down in the image and crop out an image the size of 800 width and 600 heigh.

> X-Crop: 10p10p800x600
