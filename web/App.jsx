import { createEffect, createSignal, onMount } from "solid-js";
import { createStore } from "solid-js/store";
import Background from "./Background";
import infoIcon from "./assets/img/info.png";
import { loadFileDB, saveFileDB } from "./fileUtils";
import { animationInfo as getAnimationInfo } from "./imageUtils";
import { Slider } from "./slider";

const FACTORS_OF_60 = [1, 2, 3, 4, 5, 6, 10, 12, 15, 20, 30, 60];

// Constants
const LAST_FILE_KEY = "last-file-user-uploaded";
const FORM_DATA_KEY = "giftorio-form-data";
const _INITIAL_VALUES = {
	connectionDirection: "horizontal",
	file: null,
	flippedAxes: "none",
	grayscaleBits: 0,
	imageRotation: "none",
	includeLastFrame: false,
	maxSize: 50,
	outputFormat: "blueprint",
	resamplingFilter: "triangle",
	rotation: 0,
	signalCompressionType: "none",
	sortSignals: false,
	substationQuality: "normal",
	targetFps: 15,
	temporalCompressionWindow: 300,
	useDLC: false,
	wireColor: "green",
};

function setInitialValues(values) {
	localStorage.setItem(FORM_DATA_KEY, JSON.stringify(values));
}
const INITIAL_VALUES = (() => {
	const prev = localStorage.getItem(FORM_DATA_KEY);
	if (prev) return JSON.parse(prev);

	setInitialValues(_INITIAL_VALUES);
	return { ..._INITIAL_VALUES };
})();

const FORM_ELEMENTS = {
	useDLC: {
		name: "Use Space Age DLC?",
		type: "checkbox",
		tooltip: `
			If enabled, dramatically increases the number of available signals,
			reducing the number of combinators in the blueprint by ~15x.
			It also allows for higher quality substations.
			<br/>
			<br/>
			<span class='text-tan-500' style='opacity:0.5;'>
				Requires the Space Age DLC (v2.0.77 or later).
			</span>
		`,
	},
	substationQuality: {
		name: "Substation Quality",
		type: "select",
		options: [
			["normal", "Normal"],
			["uncommon", "Uncommon"],
			["rare", "Rare"],
			["epic", "Epic"],
			["legendary", "Legendary"],
			["none", "None"],
		],
		tooltip: `
			The quality level of the substations.
			<br/>
			<br/>
			<span class='text-tan-500' style='opacity:0.5;'>
				Usable if <strong>Use Space Age DLC?</strong> is enabled.
			</span>
		`,
	},
	signalCompressionType: {
		name: "Signal Compression",
		type: "select",
		tooltip: `
			Signal compression is a <strong>lossless</strong> compression method that reduces blueprint
			size by storing unchanged pixels.
			<br/>
			<br/><strong>Temporal Compression</strong>
			<br/>Reduces filesize up to 75% depending on the source video, but uses more combinators.
			<br/>
			<br/>
			<span class='text-tan-500' style='opacity:0.5;'>
				Temporal compression scans the frames on a fixed window and stores any non-changing
				pixels within the last window using one combinator, instead of always storing every
				pixel in a combinator per-frame.
				Temporal compression is <strong>non-volatile</strong>, and the resulting blueprint can
				be seeked to any point in time safely without corruption.
			</span>
			<br/>
			<br/><strong>Delta Compression</strong>
			<br/>Has better compression than temporal compression, BUT the resulting blueprint cannot be
			seeked / paused and must only be played from start to finish. Delta signals are reset on the
			first frame.
			<br/>
			<br/>
			<span class='text-tan-500' style='opacity:0.5;'>
				Delta compression uses a memory combinator to hold all signal values, allowing the data
				combinators to only store the difference in pixel values between frames rather than the
				real values. This means any non-changing pixels can be ommited from the output, but a
				frame requires state from all previous frames to render, so the resulting blueprint
				cannot be paused or seeked, and must only be played from start to finish.
			</span>
			<br/>
			<br/><strong>None</strong>
			<br/>No compression.
		`,
		options: {
			none: "None",
			temporal: "Temporal (static)",
			delta: "Delta (volatile)",
		},
	},
	temporalCompressionWindow: {
		name: "Compression Window (ms)",
		type: "number",
		tooltip: `
			Scans all pixels every <strong>Window (ms)</strong> milliseconds, finds all the pixels that
			did not change in the last scan, and stores them in a single combinator. The other pixels
			(that changed) are stored in per-frame combinators.
			<br/>
			<br/>
			<span class='text-tan-500' style='opacity:0.5;'>
				This setting should be fine-tuned to the amount of movement in the GIF. Larger windows
				can give greater compression, but if large portions of the GIF are moving, less
				compression is possible in comparison to using shorter sampling times.
				<br/>
				<br/>
				Only applicable to <strong>Temporal Compression</strong>.
			</span>
		`,
		min: 100,
		max: 5000,
		step: 100,
	},
	targetFps: {
		name: "Framerate (FPS)",
		type: "number",
		tooltip: `
			Maximum framerate of the output blueprint.
			<br/>
			<br/>
			The blueprint will not exceed the original framerate of the GIF. Higher framerates require
			more frames to be generated, increasing the size of the blueprint.
			<br/>
			<br/>
			<span class='text-tan-500' style='opacity:0.5;'>
				Must be a factor of <strong>60</strong> (to match Factorio's tick-rate).
			</span>
		`,
		values: FACTORS_OF_60,
	},
	includeLastFrame: {
		name: "Always Include Last Frame",
		type: "checkbox",
		tooltip: `
			If enabled, the last frame of the GIF will always be included. May cause the GIF to stutter
			when looping.
			<br/>
			<br/>
			<span class='text-tan-500' style='opacity:0.5;'>
				Usually should be disabled unless it is desirable to see the last frame of the GIF.
			</span>
		`,
	},
	sortSignals: {
		name: "Sort Signals",
		type: "checkbox",
		tooltip: `
			If enabled, sorts signals by the combined string length of their internal 'type' and 'name'
			fields. This helps reduce the size of the blueprint for small group sizes.
			<br/>
			<br/>
			If disabled, signals are sorted how their appear in Factorio.
			<br/>
			<br/>
			<span class='text-tan-500' style='opacity:0.5;'>
				Usually recommended, but note this setting also lamp signal ordering and may clash with
				another display if the same display for multiple GIFs.
			</span>
		`,
	},
	grayscaleBits: {
		name: "Colour Mode",
		type: "select",
		tooltip: `
			Full colour will try to match the original GIF colours.
			<br/>
			<br/>
			Greyscale reduces the size of the blueprint by 75-95%. Greyscale is also recommended for
			black-and-white GIFs.
			<br/>
			<br/>
			8-bit greyscale has 256 shades of grey and can reduce the blueprint size by 60-70%; 4-bit
			greyscale has 16 shades of grey and can reduce the blueprint size by up to 85%. Full black
			and white is ~32x smaller than full colour.
		`,
		options: [
			["0", "Full Colour"],
			["8", "8-bit Greyscale (256)"],
			["4", "4-bit Greyscale (16)"],
			["1", "Black & White"],
		],
	},
	resamplingFilter: {
		name: "Resampling Filter",
		type: "select",
		tooltip: `
			The filter used to resample the image.
			<br/>
			<br/><strong>Triangle</strong>
			<br/>Fast, CPU friendly filter. Great for most videos, but has lower quality than other
			filters.
			<br/>
			<br/><strong>Catmull-Rom</strong>
			<br/>Has good sharpness and stability, may slightly blur. Gives the most consistent
			results.
			<br/>
			<br/><strong>Lanczos3</strong>
			<br/>More detailed, sharper edges but can shimmer. Yields crisp images, but may have minor
			aliasing.
			<br/>
			<br/><strong>Gaussian</strong>
			<br/>Very smooth outputs, but can blur pixel art. Very stable, no aliasing.
			<br/>
			<br/><strong>Nearest</strong>
			<br/>Generates risp outputs but looks "blocky". Great for pixel art, especially when the
			dimensions are matched up.
			<br/>
			<br/>
			<span class='text-tan-500' style='opacity:0.5;'>
				Only used to resize the video to the blueprint's dimensions.
			</span>
		`,
		options: {
			triangle: "Triangle",
			catrom: "Catmull-Rom",
			gaussian: "Gaussian",
			lanczos3: "Lanczos3",
			nearest: "Nearest",
		},
	},
	imageRotation: {
		name: "Rotation",
		type: "select",
		tooltip: `
			Rotates the GIF before processing it, whilst keeping data combinators in the same location.
			Combinators will always appear in the top-left corner.
			<br/>
			<br/>
			<span class='text-tan-500' style='opacity:0.5;'>
				This can be utilised to change the location of data combinators. E.g. to put combinators
				on the left side instead of the right, set this to 90°, and then inside Factorio, rotate
				the blueprint by -90° to match the original GIF.
				<br/>
				<br/>
				<strong>Wire Connection Direction</strong> is honored.
			</span>
		`,
		options: {
			none: "None",
			deg90: "90° Clockwise",
			deg180: "180° Clockwise",
			deg270: "270° Clockwise",
		},
	},
	flippedAxes: {
		name: "Flip Axes",
		type: "select",
		tooltip: `
			Mirrors the GIF before processing it, whilst keeping data combinators in the same location.
			Combinators will always appear in the top-left corner.
			<br/>
			<br/>
			<span class='text-tan-500' style='opacity:0.5;'>
				This can be utilised to change the location of data combinators. E.g. to put combinators
				on the right of the top side instead of the right, set this to <strong>X</strong>, and
				then inside Factorio, flip the blueprint horizontally to match the original GIF.
			</span>
		`,
		options: {
			none: "None",
			x: "X",
			y: "Y",
			both: "Both",
		},
	},
	wireColor: {
		name: "Preferred Wire Colour",
		type: "select",
		tooltip: `
			The colour of the wires used to connect the lamps.
			<br/>
			<br/> <strong>Green</strong> is usually recommended as green wires connect horizontally in
			straight lines and take up the least space.
			<br/><strong>Red</strong> wires are darker and harder to see but take up more screen space
			as the wire does not connect straight, and may obscure the video more.
			<br/>
			<br/>
			<span class='text-tan-500' style='opacity:0.5;'>
				The wire colour used will slightly tint the image the same colour. Note that changing
				this setting will completely flip all wires (all red wires become green, all green wires
				become red, and vice versa). There will always be a wire of the opposite colour
				connecting the lamps together at the top row to transmit data.
			</span>
		`,
		options: {
			green: "Green",
			red: "Red",
		},
	},
	connectionDirection: {
		name: "Wire Connection Direction",
		type: "select",
		tooltip: `
			Whether the majority of lamp wires should connect horizontally or vertically.
			<br/>
			<br/><strong>Horizontal</strong>
			<br/>Recommended as they take minimal screen space, but require vertical connections between
			groups that may be quite visible. Seams can be removed by rotating the blueprint 90°
			clockwise.
			<br/>
			<br/><strong>Vertical</strong>
			<br/>(Not recommended) Connections are much more noticeable.
			<br/>
			<br/>Red wires only connect straight vertically.
			<br/>Green wires connect straight both horizontally and vertically.
			<br/>
			<br/>
			<span class='text-tan-500' style='opacity:0.5;'>
				Horizontal wires are recommended for most GIFs as they are more obscure.
				<br/>
				<br/>
				If <strong>Rotation</strong> is set, the wires will still connect in the same direction
				relative to the original GIF's rotation.
			</span>
		`,
		options: {
			horizontal: "Horizontal",
			vertical: "Vertical",
		},
	},
	outputFormat: {
		name: "Output Format",
		type: "select",
		tooltip: `
			Selects the format of the output file.
			<br/>
			<br/><strong>Blueprint</strong>
			<br/>The typical Factorio blueprint format. Can be copy-pasted into the 'Import Blueprint
			String' dialog in the Factorio editor.
			<br/>
			<br/><strong>JSON</strong>
			<br/>The raw JSON file containing the blueprint data. From Factorio v2.0.25 onwards, JSON
			and blueprint files can be imported directly into the game via drag-and-drop.
			<br/>
			<br/>
			<span class='text-tan-500' style='opacity:0.5;'>
				<strong>
					If Factorio has issues importing blueprint strings, try using the JSON format.
				</strong>
				<br/>
				JSON format requires Factorio v2.0.25 or later.
			</span>
		`,
		options: {
			blueprint: "Blueprint",
			json: "Raw JSON",
		},
	},
};

function formatDuration(ms) {
	const totalSeconds = Math.floor(ms / 1000);

	const hours = Math.floor(totalSeconds / 3600);
	const minutes = Math.floor((totalSeconds % 3600) / 60);
	const seconds = totalSeconds % 60;
	const milliseconds = ms % 1000;

	return (
		`${String(hours).padStart(2, "0")}:` +
		`${String(minutes).padStart(2, "0")}:` +
		`${String(seconds).padStart(2, "0")}.` +
		`${String(milliseconds).padStart(3, "0")}`
	);
}

function App({ worker }) {
	// State
	const [formData, setFormData] = createStore({ ...INITIAL_VALUES });
	const [animationInfo, setAnimationInfo] = createSignal(null);
	const [isGenerating, setIsGenerating] = createSignal(false);
	const [needsTooltipUpdate, setNeedsTooltipUpdate] = createSignal(true);
	const [progress, setProgress] = createSignal({ percentage: 0.0, status: "Starting..." });
	const [toast, setToast] = createSignal({ show: false, message: "", isError: false });
	const [isDragging, setIsDragging] = createSignal(false);
	const [xOffset, setXOffset] = createSignal(0);
	const [yOffset, setYOffset] = createSignal(0);
	const [showAdvanced, setShowAdvanced] = createSignal(true);
	const [isMobile, setIsMobile] = createSignal(false);
	let form;

	// Refs
	let formRefs = {};

	// Worker message handler
	worker.onmessage = async (event) => {
		if (event.data.progress) {
			const { percentage, status } = event.data.progress;

			setProgress({ percentage, status });
			formRefs.progressBar.style.width = `${percentage}%`;
			formRefs.progressStatus.textContent = status;
		} else if (event.data.blueprintMetadata) {
			const { blueprintMetadata } = event.data;

			setToast({
				show: true,
				message: "Blueprint downloaded! If you're having trouble importing the blueprint into Factorio, try using the JSON format.",
				isError: false,
			});
			setTimeout(() => setToast({ show: false, message: "", isError: false }), 3000);

			formRefs.progressContainer.classList.add("hidden");
			formRefs.blueprintResult.classList.remove("hidden");
			formRefs.responseText.innerHTML = "Blueprint downloaded!";
			formRefs.submitButton.disabled = false;
		} else if (event.data.error) {
			setToast({ show: true, message: event.data.error, isError: true });
			setTimeout(() => setToast({ show: false, message: "", isError: false }), 3000);
			setIsGenerating(false);
			formRefs.submitButton.disabled = false;
		}
	};

	function setInputFile(file) {
		setFormData("file", file);
		file.arrayBuffer().then((buffer) => {
			setAnimationInfo(getAnimationInfo(new Uint8Array(buffer)));
		});
	}

	async function handleSubmit(event) {
		event.preventDefault();

		// Validate ALL inputs (instead of only showing errors for the first invalid input)
		let isValid = true;
		for (const e of [event.target, ...Object.values(formRefs)]) {
			if (e.checkValidity?.() === false) {
				e.reportValidity();
				isValid = false;
			}
		}
		if (!isValid) {
			setToast({ show: true, message: "Please fix all errors", isError: true });
			setTimeout(() => setToast({ show: false, message: "", isError: false }), 3000);
			return;
		}

		setIsGenerating(true);
		formRefs.submitButton.disabled = true;

		// Reset UI state
		formRefs.blueprintStatus.classList.remove("hidden");
		formRefs.progressContainer.classList.remove("hidden");
		formRefs.blueprintResult.classList.add("hidden");
		setProgress({ percentage: 0, status: "Starting..." });

		// Reset progress bar and status text explicitly
		formRefs.progressBar.style.width = "0%";
		formRefs.progressStatus.textContent = "Starting...";

		if (!formData.file) {
			setToast({ show: true, message: "Please select a file", isError: true });
			setTimeout(() => setToast({ show: false, message: "", isError: false }), 3000);
			setIsGenerating(false);
			formRefs.submitButton.disabled = false;
			return;
		} else if (formData.file.type !== "image/gif" && formData.file.type !== "image/webp") {
			setToast({ show: true, message: "Please select a GIF/WebP file", isError: true });
			setTimeout(() => setToast({ show: false, message: "", isError: false }), 3000);
			setIsGenerating(false);
			formRefs.submitButton.disabled = false;
			return;
		} else {
			saveFileDB(formData.file, LAST_FILE_KEY);
		}

		try {
			const imageData = new Uint8Array(await formData.file.arrayBuffer());
			let signalCompression = null;

			if (formData.signalCompressionType === "delta") {
				signalCompression = "delta";
			} else if (formData.signalCompressionType === "temporal") {
				signalCompression = {
					temporal: {
						window: +formData.temporalCompressionWindow,
					},
				};
			}

			worker.postMessage({
				generate: {
					imageData,
					args: {
						flippedAxes: `${formData.flippedAxes}`,
						grayscaleBits: +formData.grayscaleBits,
						imageRotation: `${formData.imageRotation}`,
						imageType: formData.file.type.substring(6 /* image/ */),
						includeLastFrame: !!formData.includeLastFrame,
						maxSize: +formData.maxSize,
						name: `${formData.file.name}`,
						outputFormat: `${formData.outputFormat}`,
						resamplingFilter: `${formData.resamplingFilter}`,
						rotation: +formData.rotation,
						signalCompression,
						sortSignals: !!formData.sortSignals,
						substationQuality: `${formData.substationQuality}`,
						targetFps: +formData.targetFps,
						useDLC: !!formData.useDLC,
						useGreenLampWires: !!(formData.wireColor === "green"),
						useHorizontalLampWires: !!(formData.connectionDirection === "horizontal"),
					},
				},
			});
		} catch (err) {
			console.error("Failed to process file:", err);
			setToast({ show: true, message: "Failed to process file", isError: true });
			setTimeout(() => setToast({ show: false, message: "", isError: false }), 3000);
			setIsGenerating(false);
			formRefs.submitButton.disabled = false;
		}
	}

	function handleMouseDown(e) {
		e.preventDefault();
		setXOffset(e.clientX - form.getBoundingClientRect().left);
		setYOffset(e.clientY - form.getBoundingClientRect().top);
		e.target.style.cursor = "grabbing";
		setIsDragging(true);

		document.addEventListener("mouseup", handleMouseUp);
	}

	function handleMouseUp(e) {
		if (isDragging()) {
			e.preventDefault();
			e.target.style.cursor = "pointer";
			setIsDragging(false);
		}

		document.removeEventListener("mouseup", handleMouseUp);
	}

	document.addEventListener("mousemove", (e) => {
		if (isDragging()) {
			form.style.position = "absolute";
			form.style.left = `${e.clientX - xOffset()}px`;
			form.style.top = `${e.clientY - yOffset()}px`;
		}
	});

	onMount(() => {
		// Check if device is mobile
		setIsMobile(window.innerWidth <= 768);
		window.addEventListener("resize", () => {
			setIsMobile(window.innerWidth <= 768);
		});

		const bounds = form.getBoundingClientRect();
		form.style.position = "absolute";
		form.style.left = `${bounds.left}px`;
		form.style.top = `${bounds.top}px`;
	});

	createEffect(() => {
		if (!needsTooltipUpdate()) return;

		document.querySelectorAll(".tooltip-trigger").forEach((trigger) => {
			trigger.addEventListener("mousemove", (e) => {
				const tooltip = trigger.nextElementSibling;
				const rect = trigger.getBoundingClientRect();

				const vpWidth = window.innerWidth;
				const vpHeight = window.innerHeight;
				const tooltipRect = tooltip.getBoundingClientRect();

				let x = e.clientX + 10;
				let y = e.clientY + 10;

				// Check if tooltip would go off-screen to the right
				if (x + tooltipRect.width > vpWidth) {
					x = e.clientX - tooltipRect.width - 10;
				}

				// Check if tooltip would go off-screen at the bottom
				if (y + tooltipRect.height > vpHeight) {
					y = e.clientY - tooltipRect.height - 10;
				}

				tooltip.style.left = `${x}px`;
				tooltip.style.top = `${y}px`;
			});
		});

		setNeedsTooltipUpdate(false);
	});

	onMount(() => {
		loadFileDB(LAST_FILE_KEY)
			.then((file) => {
				if (file) {
					const dataTransfer = new DataTransfer();
					dataTransfer.items.add(file);

					formRefs.fileInput.files = dataTransfer.files;
					setInputFile(file);
				}
			})
			.catch((err) => {
				console.error("Failed to load file:", err);
				console.error("Failed to load file:", err);
				setToast({ show: true, message: "Failed to load file", isError: true });
			});
	});

	createEffect(() => {
		if (!formData.useDLC && !["none", "normal"].includes(formData.substationQuality)) {
			setFormData("substationQuality", "normal");
		}
		setInitialValues(formData);
	});

	return (
		<>
			<Background />
			{isMobile() && (
				<div class="mobile-warning">⚠️ GIFtorio works best on desktop devices. Some features may be limited on mobile.</div>
			)}
			<div class="flex flex-col items-center justify-center min-h-screen">
				<div
					classList={{
						"opacity-0": !toast().show,
						"opacity-100": toast().show,
						"bg-green-500": !toast().isError,
						"bg-red-500": toast().isError,
					}}
					class="fixed top-4 right-4 text-white px-4 py-2 rounded shadow-lg transition-opacity duration-300"
				>
					{toast().message}
				</div>

				<div ref={form} class="panel-container flex z-1">
					<div classList={{ hidden: isGenerating() }} class="panel form flex-shrink-0">
						<div class="flex items-center justify-between">
							<h2 class="text-tan-500">Convert GIF (or WebP) to Blueprint</h2>
							<div class="handle cursor-pointer" onMouseDown={handleMouseDown}></div>
							<div
								class="mb-[10px] w-5 h-5 flex items-center content-center justify-center"
								classList={{
									"panel-inset-orange": showAdvanced(),
									"text-black": showAdvanced(),
									"panel-inset-light": !showAdvanced(),
									"text-white": !showAdvanced(),
								}}
								onClick={() => setShowAdvanced(!showAdvanced())}
							>
								<svg
									xmlns="http://www.w3.org/2000/svg"
									fill="none"
									viewBox="0 0 24 24"
									stroke-width="1.5"
									stroke="currentColor"
									class="size-4"
								>
									<path
										stroke-linecap="round"
										stroke-linejoin="round"
										d="M9.594 3.94c.09-.542.56-.94 1.11-.94h2.593c.55 0 1.02.398 1.11.94l.213 1.281c.063.374.313.686.645.87.074.04.147.083.22.127.325.196.72.257 1.075.124l1.217-.456a1.125 1.125 0 0 1 1.37.49l1.296 2.247a1.125 1.125 0 0 1-.26 1.431l-1.003.827c-.293.241-.438.613-.43.992a7.723 7.723 0 0 1 0 .255c-.008.378.137.75.43.991l1.004.827c.424.35.534.955.26 1.43l-1.298 2.247a1.125 1.125 0 0 1-1.369.491l-1.217-.456c-.355-.133-.75-.072-1.076.124a6.47 6.47 0 0 1-.22.128c-.331.183-.581.495-.644.869l-.213 1.281c-.09.543-.56.94-1.11.94h-2.594c-.55 0-1.019-.398-1.11-.94l-.213-1.281c-.062-.374-.312-.686-.644-.87a6.52 6.52 0 0 1-.22-.127c-.325-.196-.72-.257-1.076-.124l-1.217.456a1.125 1.125 0 0 1-1.369-.49l-1.297-2.247a1.125 1.125 0 0 1 .26-1.431l1.004-.827c.292-.24.437-.613.43-.991a6.932 6.932 0 0 1 0-.255c.007-.38-.138-.751-.43-.992l-1.004-.827a1.125 1.125 0 0 1-.26-1.43l1.297-2.247a1.125 1.125 0 0 1 1.37-.491l1.216.456c.356.133.751.072 1.076-.124.072-.044.146-.086.22-.128.332-.183.582-.495.644-.869l.214-1.28Z"
									/>
									<path stroke-linecap="round" stroke-linejoin="round" d="M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z" />
								</svg>
							</div>
						</div>
						<form onSubmit={handleSubmit} class="panel-inset-light bg-gray-500 p-6 rounded shadow-md w-full max-w-md">
							<div class="mb-4 flex items-center justify-between">
								{/* File Input */}
								<div class="">
									<input
										ref={(el) => (formRefs.fileInput = el)}
										class="text-white-500 w-full focus:outline-none focus:ring"
										type="file"
										id="gifInput"
										required
										onChange={(e) => setInputFile(e.target.files[0])}
										accept="image/gif,image/webp"
									/>
								</div>
							</div>

							{animationInfo() && (
								<div>
									<div class="text-gray-300 font-semibold">
										{formatDuration(animationInfo().duration)} ({animationInfo().frames} Frames, ~
										{Math.round((10 * (animationInfo().frames * 1000)) / animationInfo().duration) / 10} FPS)
									</div>
								</div>
							)}

							{/* Max Size Input */}
							<div class="mt-5 mb-4 flex items-center justify-between">
								<label class="text-white-500" for="maxsize">
									Max Size
									<img src={infoIcon} class="inline-block ml-1 mb-0.5 w-4 h-4 tooltip-trigger" alt="Info" />
									<span class="tooltip">
										Maximum size of the longest side (length or width) of the output image in tiles.
										<br />
										<br />
										Larger values create higher resolution blueprints but take longer to generate and import, and can
										negatively impact on game performance.
										<br />
										<br />
										Blueprint size increases x4 for a x2 increase in max size.
									</span>
								</label>

								<input
									ref={(el) => (formRefs.maxsize = el)}
									class="bg-gray-100 focus:bg-tan-500 w-24 px-3 py-2 border rounded focus:outline-none focus:ring"
									type="number"
									id="maxsize"
									onInput={(e) => setFormData("maxSize", e.target.value)}
									value={formData.maxSize}
									min="2"
									max="1080"
								/>
							</div>

							{/* Advanced Settings and Submit Buttons */}
							<div>
								<div class="flex items-center justify-between">
									<button class="button bg-gray-100 px-4" type="button" onClick={() => setShowAdvanced(!showAdvanced())}>
										Advanced Options
									</button>
									<button
										class="button button-green-right"
										ref={(el) => (formRefs.submitButton = el)}
										id="submit"
										type="submit"
									>
										Generate
									</button>
								</div>
							</div>
						</form>
					</div>

					<div class="panel w-100 z-10" classList={{ hidden: !showAdvanced() || isGenerating() }}>
						<div class="flex items-center justify-between">
							<h3 class="text-tan-500">Advanced Options</h3>
							<div class="handle cursor-pointer" onMouseDown={handleMouseDown} onMouseUp={handleMouseUp}></div>
						</div>

						<div class="panel-inset-light p-3 shadow-md w-full max-w-md">
							{Object.entries(FORM_ELEMENTS).map(([k, v]) => {
								if (k === "substationQuality") {
									let { options } = v;
									options = formData.useDLC ? v.options : v.options.filter(([k, ..._]) => k === "none" || k === "normal");
									v = { ...v, options };
								} else if (k === "temporalCompressionWindow" && formData.signalCompressionType !== "temporal") {
									v = { ...v, disabled: true };
								}
								setNeedsTooltipUpdate(true);
								return makeFormElement({ formData, formRefs, setFormData, obj: { [k]: v } });
							})}
						</div>
					</div>

					{/* Blueprint Status Section */}
					<div
						ref={(el) => (formRefs.blueprintStatus = el)}
						classList={{ hidden: !isGenerating() }}
						class="panel w-full max-w-md min-w-[384px]"
					>
						<div ref={(el) => (formRefs.progressContainer = el)} id="progressContainer">
							<div class="w-full bg-dark-gray-500 rounded-full h-4 mb-2">
								<div
									ref={(el) => (formRefs.progressBar = el)}
									id="progressBar"
									class="bg-green-500 h-4 rounded-full"
									style="width: 0%"
								>
									{progress.percentage}%
								</div>
							</div>
							<p ref={(el) => (formRefs.progressStatus = el)} id="progressStatus" class="text-tan-500">
								Starting...
							</p>
						</div>
						<div ref={(el) => (formRefs.blueprintResult = el)} id="blueprintResult" classList={{ hidden: !isGenerating() }}>
							<h2 class="text-tan-500">Blueprint Download</h2>
							<div
								ref={(el) => (formRefs.responseText = el)}
								id="responseText"
								class="panel-inset-light text-gray-100 p-3 border rounded mb-6 overflow-y-auto overflow-x-hidden whitespace-pre-wrap break-all h-32"
							></div>
							<div class="flex items-center justify-between">
								<button onClick={() => setIsGenerating(false)} id="backButton" class="button">
									Back
								</button>
							</div>
							<div class="mt-6 text-center text-white-500">
								<p>
									Liking GIFtorio? Consider{" "}
									<a
										href="https://www.buymeacoffee.com/colinchilds"
										target="_blank"
										rel="noopener noreferrer"
										class="text-bright-green-500 hover:text-tan-500"
									>
										supporting the original creator
									</a>{" "}
									!
								</p>
							</div>
						</div>
					</div>
				</div>

				<footer class="fixed bottom-8 w-full gap-8 text-center text-gray-300">
					<a href="https://github.com/colinchilds/giftorio" class="pr-8" target="_blank" rel="noopener noreferrer">
						<svg
							xmlns="http://www.w3.org/2000/svg"
							viewBox="0 0 30 30"
							width="1em"
							height="1em"
							fill="currentColor"
							class="inline-block align-middle mr-1.5"
						>
							<path d="M15 3C8.373 3 3 8.373 3 15c0 5.623 3.872 10.328 9.092 11.63a1.8 1.8 0 0 1-.092-.583v-2.051h-1.508c-.821 0-1.551-.353-1.905-1.009-.393-.729-.461-1.844-1.435-2.526-.289-.227-.069-.486.264-.451.615.174 1.125.596 1.605 1.222.478.627.703.769 1.596.769.433 0 1.081-.025 1.691-.121.328-.833.895-1.6 1.588-1.962-3.996-.411-5.903-2.399-5.903-5.098 0-1.162.495-2.286 1.336-3.233-.276-.94-.623-2.857.106-3.587 1.798 0 2.885 1.166 3.146 1.481A9 9 0 0 1 15.495 9c1.036 0 2.024.174 2.922.483C18.675 9.17 19.763 8 21.565 8c.732.731.381 2.656.102 3.594.836.945 1.328 2.066 1.328 3.226 0 2.697-1.904 4.684-5.894 5.097C18.199 20.49 19 22.1 19 23.313v2.734c0 .104-.023.179-.035.268C23.641 24.676 27 20.236 27 15c0-6.627-5.373-12-12-12"></path>
						</svg>
						<span class="align-middle">Contribute or report an issue</span>
					</a>
					<a href="https://www.buymeacoffee.com/colinchilds" target="_blank" rel="noopener noreferrer">
						<svg
							xmlns="http://www.w3.org/2000/svg"
							viewBox="0 0 24 24"
							width="1em"
							height="1em"
							fill="currentColor"
							class="inline-block align-middle mr-1.5"
						>
							<path d="M23.881 8.948c-.773-4.085-4.859-4.593-4.859-4.593H.723c-.604 0-.679.798-.679.798s-.082 7.324-.022 11.822c.164 2.424 2.586 2.672 2.586 2.672s8.267-.023 11.966-.049c2.438-.426 2.683-2.566 2.658-3.734 4.352.24 7.422-2.831 6.649-6.916m-11.062 3.511c-1.246 1.453-4.011 3.976-4.011 3.976s-.121.119-.31.023c-.076-.057-.108-.09-.108-.09-.443-.441-3.368-3.049-4.034-3.954-.709-.965-1.041-2.7-.091-3.71.951-1.01 3.005-1.086 4.363.407 0 0 1.565-1.782 3.468-.963s1.832 3.011.723 4.311m6.173.478c-.928.116-1.682.028-1.682.028V7.284h1.77s1.971.551 1.971 2.638c0 1.913-.985 2.667-2.059 3.015"></path>
						</svg>
						<span class="align-middle">Support</span>
					</a>
				</footer>
			</div>
		</>
	);
}

function makeFormElement({ formData, formRefs, setFormData, obj }) {
	const k = Object.keys(obj)[0];
	const v = obj[k];
	const { disabled = false, name, tooltip = null, type } = v;

	function mkTooltip(text) {
		if (!text) {
			return null;
		}

		const el = document.createElement("div");
		el.classList.add("tooltip");
		el.innerHTML = text.replaceAll("\n", " ");

		return (
			<>
				<img src={infoIcon} className="inline-block ml-1 mb-0.5 w-4 h-4 tooltip-trigger" alt="Info" />
				{el}
			</>
		);
	}

	if (type === "checkbox") {
		return (
			<div class="mb-1 flex">
				<label class="checkbox-label">
					<input
						type="checkbox"
						class="sr-only"
						checked={formData[k]}
						onChange={(e) => setFormData(k, e.currentTarget.checked)}
					/>
					<div class="checkbox"></div>
					<div class="ml-3 text-white-500">
						{name}
						{mkTooltip(tooltip)}
					</div>
				</label>
			</div>
		);
	} else if (type === "select") {
		const { options } = v;
		return (
			<div class="mt-1 mb-1 flex items-center justify-between factorio-select-container">
				<label class="block text-white-500" for={k}>
					{name}
					{mkTooltip(tooltip)}
				</label>
				<select
					ref={(e) => (formRefs[k] = e)}
					id={k}
					name={k}
					class="bg-gray-100 font-semibold border focus:outline-none focus:ring"
					value={formData[k]}
					onChange={(e) => setFormData(k, e.currentTarget.value)}
				>
					{(Array.isArray(options) ? options : Object.entries(options)).map(([k, v]) => (
						<option value={k}>{v}</option>
					))}
				</select>
			</div>
		);
	} else if (type === "number") {
		function handleSubmit(rawValue) {
			if (v.values && !v.values.includes(rawValue)) {
				formRefs[k].setCustomValidity(`Value must be one of: ${v.values.join(", ")}`);
				formRefs[k].setAttribute("aria-invalid", "true");
				return;
			}
			formRefs[k].setCustomValidity("");
			formRefs[k].setAttribute("aria-invalid", "false");
			setFormData(k, rawValue);
		}
		return (
			<div class="mb-1">
				<label className="block text-white-500 mb-2" htmlFor={k}>
					{name}
					{mkTooltip(tooltip)}
				</label>
				<div class="flex items-center gap-3">
					<Slider
						id={k}
						disabled={disabled}
						ref={(e) => (formRefs[k] = e)}
						value={formData[k]}
						min={v.min}
						max={v.max}
						step={v.step}
						values={v.values}
						onSubmit={handleSubmit}
					/>
				</div>
			</div>
		);
	}
}

export default App;
