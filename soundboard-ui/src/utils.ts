export const soundNameSearchPreprocess = (s: string): string => {
	s = s.normalize("NFD").replace(/[\u0300-\u036f]/g, ""); // Remove accents and diacritics
	s = s.toLowerCase();
	s = s.replace(/[^a-z0-9 ]/g, "");
	return s;
};

export const doesNameMatchSearch = (name: string, search: string): boolean => {
	search = soundNameSearchPreprocess(search);
	name = soundNameSearchPreprocess(name);

	return name.indexOf(search) !== -1;
}

export const generateShortId = (): string => {
	const firstNum: number = Math.random() * 46656 | 0;
	const secondNum: number = Math.random() * 46656 | 0;
	const firstPart = ("000" + firstNum.toString(36)).slice(-3);
	const secondPart = ("000" + secondNum.toString(36)).slice(-3);
	return firstPart + secondPart;
}

export const setTheme = (theme: string): void => {
	document.getElementById("theme-css").setAttribute("href", `css/theme-${theme}.min.css`);
};

export const isVersionNewer = (currentVersion: string, testVersion: string): boolean => {
	const currentSplit = currentVersion.replace("v", "").split('.');
	const testSplit = testVersion.replace("v", "").split('.');

	if (currentSplit.length !== 3 || testSplit.length !== 3) {
		return false;
	}

	if (testSplit[0] > currentSplit[0]) {
		return true;
	} else if (testSplit[0] < currentSplit[0]) {
		return false;
	}
	if (testSplit[1] > currentSplit[1]) {
		return true;
	} else if (testSplit[1] < currentSplit[1]) {
		return false;
	}
	return testSplit[2] > currentSplit[2];
}
