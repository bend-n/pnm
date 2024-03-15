Decodes the body of this image, placing it in the [`Uninit`] image.

If you wish to use this yourself, use [`decode_header`] to create an [`Uninit`] image, before passing it to [`decode_body_into`].