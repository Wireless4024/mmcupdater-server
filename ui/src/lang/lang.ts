import {
	getLocaleFromNavigator,
	init,
	register
} from 'svelte-i18n';

register('en', () => import('./en'));

export function load() {
	return init({
		fallbackLocale: 'en',
		initialLocale : getLocaleFromNavigator(),
	});
}