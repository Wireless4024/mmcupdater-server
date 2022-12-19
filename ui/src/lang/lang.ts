import { register, init, getLocaleFromNavigator } from 'svelte-i18n';

register('en', () => import('./en'));
export default init({
	fallbackLocale: 'en',
	initialLocale: getLocaleFromNavigator(),
});