# wgui

Ever wondered that you would like to make web gui with rust and server-side virtual dom... probably not but here it is.


## Development

```
# Build
bun build ./ts/app.ts --watch --outfile ./dist/index.js
# Check 
bunx tsc ./ts/* --noEmit --allowImportingTsExtensions
```