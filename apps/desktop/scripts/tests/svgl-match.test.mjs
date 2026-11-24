import test from 'node:test';
import assert from 'node:assert/strict';
import { matchLogo, chooseRoutes, componentName, checkedSvgUrl } from '../lib/svgl-match.mjs';

const laravel = { title: 'Laravel', route: 'https://svgl.app/library/laravel.svg' };
const google = { title: 'Google', route: 'https://svgl.app/library/google.svg' };
const reactRouter = { title: 'React Router', route: { light: 'https://svgl.app/library/react-router-light.svg', dark: 'https://svgl.app/library/react-router-dark.svg' } };

test('associa Laravel por título exato', () => {
  const item = matchLogo({ id: 'laravel', name: 'Laravel' }, [google, laravel]);
  assert.equal(item?.entry.title, 'Laravel');
  assert.equal(item?.routes.light, laravel.route);
});
test('não atribui Google ao framework Go', () => {
  assert.equal(matchLogo({ id: 'go', name: 'Go' }, [google]), null);
});
test('aceita variantes oficiais light/dark de React Router', () => {
  const result = matchLogo({ id: 'remix', name: 'Remix / React Router' }, [reactRouter]);
  assert.equal(result?.routes.dark, 'https://svgl.app/library/react-router-dark.svg');
});
test('não aceita URLs arbitrárias', () => {
  assert.equal(checkedSvgUrl('https://evil.example/logo.svg'), null);
  assert.equal(checkedSvgUrl('http://svgl.app/library/logo.svg'), null);
  assert.equal(checkedSvgUrl('https://svgl.app/library/logo.png'), null);
});
test('aceita uma variante quando só a outra existe', () => {
  const result = chooseRoutes({ route: { dark: 'https://svgl.app/library/dark.svg' } });
  assert.equal(result.light, result.dark);
});
test('mantém IDs do LocDev sem ambiguidades', () => {
  assert.equal(componentName('solid-start'), 'SolidStart');
  assert.equal(componentName('nextjs'), 'Nextjs');
});
