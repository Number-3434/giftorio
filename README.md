# GIFtorio

[GIFtorio](https://giftor.io) is a web application that converts animated GIFs into Factorio blueprints. The resulting blueprint creates an
animated display using the game's circuit network and lamps. It requires absolutely no mods - works on vanilla Factorio version 2.0 and the
Space Age DLC. ![example nyan gif](https://github.com/colinchilds/giftorio/blob/main/web/assets/img/nyan.gif?raw=true)

## Features

- Converts animated GIFs to Factorio blueprints directly in your browser
- Automatically downscales GIFs to fit within signal limitations
- Configurable frame rate and image size
- Configurable substation quality to reduce dead pixels
- Attempts to maintain animation timing similar to the original GIF
- Supports different substation qualities when using the Space Age DLC

## Additional Features (in this fork)

- TODO: Add blueprint insertion to save files, Factorio doesn't seem to like 500 MB+ blueprint strings.
- Optimised for memory performance on longer videos. Tested with a 13,000+ frame GIF rendering at 270 max width.
- Uses chunked streaming of video frames to reduce memory pressure. (The old version had multiple buffers of the entire video which can
  destroy RAM especially on longer videos.) Uses a single streaming video pass. (As opposed to accumulating all frames into a buffer then
  processing in parallel).
- Uses 13,341 available signals from Space Age (+80% more than old version, 3 signals reserved for internal calulations)
- Max size (longest side) increased to 1,080 - so big the entire screen cannot fit in min zoom in Editor mode
- Added support for filters (nearest, triangle, catrom, lanczos3, gaussian), increasing the quality of resolution of the output.
- Remembers last settings and settings are remembered between page reloads (local storage).
- Changed clipboard copy to file download mechanic for huge blueprints (>50 MB). (Factorio 2.0.25: a file containing the blueprint text can
  be dragged into the game and Factorio will import it as a blueprint). Added progress updates.

## Prerequisites

For development:

- Rust (latest stable version)
- Node (latest LTS version)
- wasm-pack (`cargo install wasm-pack`)
- A web browser with WebAssembly support

## Development Setup

1. Clone this repository:

```bash
git clone https://github.com/colinchilds/giftorio.git
cd giftorio
```

2. Build the WebAssembly module:

```bash
wasm-pack build --target web --release
```

3. Run it with NPM:

```bash
npm install
npm start
```

4. Open your browser and navigate to `http://localhost:3000`

## Usage

1. Visit the website (or your local development server)
2. Upload your GIF file
3. Configure settings (frame rate, image size, etc.)
4. Click Generate
5. Copy the generated blueprint string
6. Import the blueprint string into Factorio

## How It Works

The application:

1. Uses WebAssembly (compiled from Rust) to process GIFs efficiently in the browser
2. Loads and downscales the input GIF to a manageable size
3. Converts each frame into a series of circuit network signals
4. Creates a blueprint containing:
    - A grid of substations to power the display
    - Constant combinators to store pixel data
    - Decider combinators to control frame timing
    - A grid of lamps to display the image
5. Outputs an encoded blueprint string compatible with Factorio

## Limitations

- Maximum image size is limited by available signals, but more realistically by in-game performance.
- Higher resolution images will require more in-game entities and may impact performance
- Browser must support WebAssembly
- Longer GIFs can take a really long time to process and may cause the game to lag. We recommend trying keep gifs to only a few seconds. If
  you have a really long gif, consider using the grayscale option, as it can signficantly reduce blueprint size.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Credits

- [@colinchilds](https://github.com/colinchilds) for the original [GIFtorio](https://github.com/colinchilds/giftorio) project

## License

[MIT License](LICENSE)
