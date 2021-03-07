import { Sound } from "./config.js";
import { SocketWrapper } from "./socketWrapper.js";
import { setTheme, soundNameSearchPreprocess, doesNameMatchSearch } from "./utils.js";

$(() => {
	onWindowOpened();

	const socket = new SocketWrapper("ws://127.0.0.1:29816");
	socket
		.onOpen(() => {
			socket.send({ "message": "clientType", "clientType": "soundboard" });
			socket.send({ "message": "sounds" });
		})
		.on("sounds", (data) => loadSounds(data.sounds))
		.on("modifiedSound", (data) => editSound(data.previousPath, data.sound))
		.on("addedSounds", (data) => addSounds(data.sounds))
		.on("removedSound", (data) => removeSound(data.path))
		.on("theme", (data) => setTheme(data.theme))
		.on("soundboardOpened", onWindowOpened)
		.on("error", (data) => console.error(data.error));

	$("#input-search").on("input", () => {
		const search: string = getSearchFilter();
		const $allSounds = $(".sound").toArray().map(el => $(el));
		if (search === "") {
			buildSoundsGrid($allSounds);
		} else {
			const $sounds = $allSounds.filter($sound => {
				const name = $sound.data("sound-name") as string;
				return doesNameMatchSearch(name, search);
			});
			buildSoundsGrid($sounds);
		}

		const $firstSound = $("#sounds .sound").first();
		if ($firstSound.length !== 0) {
			selectSound($firstSound);
		}
	});

	$(document).on("keydown", (ev) => {
		if (ev.key === "Escape") {
			socket.send({ "message": "closeSoundboard" });
		} else if (ev.key.indexOf("Arrow") !== -1) {
			const $selected = $("#sounds .sound.selected");

			if ($selected.length !== 0) {
				if (ev.key === "ArrowRight") {
					selectSoundRight($selected);
				} else if (ev.key === "ArrowLeft") {
					selectSoundLeft($selected);
				} else if (ev.key === "ArrowUp") {
					selectSoundUp($selected);
				} else if (ev.key === "ArrowDown") {
					selectSoundDown($selected);
				}
			} else {
				selectSound($("#sounds .sound").first());
			}
		} else if (ev.key === "Enter") {
			const $selected = $(".sound.selected");
			if ($selected.length !== 0) {
				socket.send({
					"message": "playSound",
					"sound": $selected.data("sound-path"),
				});
			}
		}
	});

	function onWindowOpened(): void {
		$("#input-search")
			.trigger("focus")
			.trigger("select");
	}

	function buildSound(sound: Sound): JQuery<HTMLElement> {
		const $box = $("<div>").addClass("box sound");
		$box.attr("data-sound-name", sound.name);
		$box.attr("data-sound-path", sound.path);
		if (sound.image) {
			if (sound.image.url) {
				$box.css("--background", `url(${sound.image.url})`);
			} else if (sound.image.file) {
				// TODO
				$box.css("--background", `url(http://127.0.0.1:29815/sounds/img/${sound.name})`);
			}
		}
		$box.append(`<div class="content">${sound.name}</div>`);
		$box.on("mouseenter", () => {
			selectSound($box);
		}).on("mouseleave", () => {
			$box.removeClass("selected");
		}).on("click", () => {
			socket.send({
				"message": "playSound",
				"sound": sound.path,
			});
		});
		$box.appendTo("#all-sounds");
		return $box;
	}

	function loadSounds(sounds: Sound[]) {
		const $sounds = sounds.map(sound => buildSound(sound));
		buildSoundsGrid($sounds);
	}

	function editSound(prevPath: string, newSound: Sound): void {
		const $sounds = $(".sound");
		const sound = $sounds.toArray().find(s => $(s).data("sound-path") === prevPath);
		if (!sound) {
			return;
		}
		const $sound = $(sound);
		$sound.replaceWith(buildSound(newSound));
	}

	function addSounds(sounds: Sound[]): void {
		const $sounds = $("#sounds .sound").toArray().map(el => $(el));
		const search = getSearchFilter();
		let rebuildGrid = false;

		for (const sound of sounds) {
			const $sound = buildSound(sound);
			if (doesNameMatchSearch(sound.name, search)) {
				$sounds.push($sound);
				rebuildGrid = true;
			} else {
				$("#all-sounds").append($sound);
			}
		}

		if (rebuildGrid) {
			buildSoundsGrid($sounds);
		}
	}

	function removeSound(path: string): void {
		const $sounds = $(".sound");
		const sound = $sounds.toArray().find(s => $(s).data("sound-path") === path);
		if (!sound) {
			return;
		}
		const $sound = $(sound);
		$sound.remove();
		buildSoundsGrid($("#sounds .sound").toArray().map(el => $(el)));
	}

	function buildSoundsGrid($sounds: JQuery<HTMLElement>[]) {
		$(".sound").appendTo("#all-sounds");
		$("#sounds").empty();

		let $row: JQuery<HTMLElement>;
		$sounds = $sounds.sort(($a, $b) => $a.data("sound-name").localeCompare($b.data("sound-name")));
		for (let i = 0; i < $sounds.length; ++i) {
			if (i % 5 === 0) {
				$row = $("<div>")
					.addClass("columns")
					.appendTo("#sounds");
			}

			const $col = $("<div>").addClass("column is-one-fifth").appendTo($row);
			$sounds[i].appendTo($col);
		}
	}

	function selectSound($sound: JQuery<HTMLElement>) {
		$(".sound.selected").removeClass("selected");
		$sound.addClass("selected")
	}

	function selectSoundLeft($sound: JQuery<HTMLElement>) {
		const idx = $sound.parent().index();
		selectSound($sound.parent().parent().children().eq(idx - 1).find(".sound"));
	}

	function selectSoundRight($sound: JQuery<HTMLElement>) {
		const idx = $sound.parent().index();
		const $siblings = $sound.parent().parent().children();
		selectSound($siblings.eq((idx + 1) % $siblings.length).find(".sound"));
	}

	function selectSoundUp($sound: JQuery<HTMLElement>) {
		const idx = $sound.parent().index();
		const $row = $sound.parent().parent();
		const $prevRow = $row.parent().children().eq($row.index() - 1);
		selectSound($prevRow.children().eq(idx).find(".sound"));
	}

	function selectSoundDown($sound: JQuery<HTMLElement>) {
		const idx = $sound.parent().index();
		const $row = $sound.parent().parent();
		const $siblings = $row.parent().children();
		const $nextRow = $siblings.eq(($row.index() + 1) % $siblings.length);
		const $nextRowChildren = $nextRow.children();
		selectSound($nextRowChildren.eq(Math.min(idx, $nextRowChildren.length - 1)).find(".sound"));
	}

	function getSearchFilter(): string {
		let search: string = $("#input-search").val() as string;
		search = search.trim();
		if (search !== "") {
			search = soundNameSearchPreprocess(search);
		}
		return search;
	}
});
