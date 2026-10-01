const {greeting} = require('../src/greeting');
test('greets Oyzu & preserves <characters>', () => {
  expect(greeting('Oyzu')).toBe('Hello, Oyzu!');
});
test.skip('explicitly skipped native test', () => {});
test.todo('native todo remains skipped');
