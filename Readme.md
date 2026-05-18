# dead-links

Find dead links in a github pages site.

## Cache

Because parsing big md files can take some time, dead-links caches the results, in
a file called `SITE_ROOT/.dead-links-cache`. On subsequent runs, it checks the content
hashes of the `.md` files against the one in the cache for that file, and if they are
equal, that means that the file hasn't been changed, and the diagnostics will just be
printed directly. Only updated files will be re-scanned, making subsequent runs very
fast.


## Notes

The search for links is just a regex, so it may report diagnostics for "links" it finds
matching this, regardless of if they are in a string or code block.


## Unreachable

The common case is to go from the root, and check that all documents are reachable.

Other entry points other than root/index.md ??

Unreachable check is only done when given the root of the site.

- Because if a non-root is given, what would it even do? It would only make sense
  to still do the unreachability check from the root, but then only report on unreacahble
  files in the directory you're inside. But doing the unreachability check would mean
  scanning the entire site, which is expensive, and when a subdir is given, you're
  doing it because it's faster (with less output).

- strange to have a site where not all files are deliberately not reachable?
  .. can support that use-case, but with an ignore system, e.g. a file with
  like `PATH/TO/FILE ignore` (disable all checking), or `ignore-unreachable`.

- "multi universe sites"? Where the index only reaches a subset, and then, another
  "index" reaches the other subset? Don't want to support.

- If root of site not given as input:
  - still do unreachable check from the index, but only report on files that are
    unreachable in the given directory?
    OR
    just don't do the unreachable check when not given root.

BUT:
  - May not always give in the root (if only looking at subdirs)
    In that case, it AT LEAST should not give any unreachable information for
    directories OUTSIDE that given directory.
    ... No, it doesn't make sense to ask for reachability outside of the root, ..right?
