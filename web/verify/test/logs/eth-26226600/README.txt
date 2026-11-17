Data dir for test/browser.mjs and test/real-proofs.mjs: Ethereum blocks 26226600-26226601 (40 blocks into
their 64-block window, as the download cases need), each proven on its own (one-block parts), saved
with test/save_live_data.py. LEAF_KEY is the published block-program key that proved them (the page's
Published keys).
Run from web/verify: PLAYWRIGHT=<dir> node test/browser.mjs <dist> test/logs/eth-26226600 $(cat test/logs/eth-26226600/LEAF_KEY)
