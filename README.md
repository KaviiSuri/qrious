# QRious

I was curious to understand how QR codes work, so I made this. It's a simple QR code reader. 


## Roadmap
- [x] Load image
- [x] Setup a DevTools for debugging using svg
- [x] Detect finder patterns
- [x] Detect timing patterns
- [x] Detect version
- [x] Detect format information
- [x] Detect data
- [x] Decode data
- [x] Refactor code to creat an iterator for bytes
- [ ] Try other data encodings
- [ ] Try other QA code versions
- [ ] Refactor the qr.rs module to a folder, it's getting insane
- [ ] Display data

> The roadmap is subject to change as I learn more about QR codes. It's just a list of things I might do in no particular order.

## Disclaimer

This is a work in progress. I'm learning as I go. I'm not an expert. I'm just curious.

## Credits

I'm primarily following [this](https://www.youtube.com/playlist?list=PL980gcR1LE3KsjJ3EsybETRYqrH0iQPwb), sometimes blatantly copying the approach to get things working when I'm stuck. The [wikipedia page](https://en.wikipedia.org/wiki/QR_code) is also a great resource.
And also, the [QR Step by Step](https://www.nayuki.io/page/creating-a-qr-code-step-by-step) is pretty cool.

## Questions Noone Asked

### How to run this?

```bash
cargo run ./path/to/qr-code.png ./path/to/output-dir
```

### Will this work with all QR codes?

Most definitely not. This is a very simple implementation. I haven't handled any real world edge case like noise or rotation, etc. I also haven't implemented any error correction. This is just a simple implementation to understand how this amazing technology, that we take for granted, works.

### Why are there no tests?

I'm learning as I go. I'm not sure how to test this yet. I also keep refactoring the code as I learn more, will add tests once I'm more confident in the code.

### Why is this in Rust?

Cuz Rust is fun. I wanna learn Rust. I'm not sure if Rust is the best language for this, but I'm learning Rust, so I'm using Rust.

### What's next?

I'm not sure. Just playing around right now, wanna explore mounting this library in a react-native app. Maybe I'll do that next. Who knows?

