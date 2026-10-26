### Fixed

- File drops stop reading at the size limit. A file that grows after its metadata is checked, or that has no usable size, is cut off by a bounded reader instead of being read in full and then refused.
