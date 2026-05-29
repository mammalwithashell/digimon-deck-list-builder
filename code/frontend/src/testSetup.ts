/// <reference types="@testing-library/jest-dom" />
//
// Vitest setup file: ensures `@testing-library/react` cleans up the DOM
// after every test so consecutive `render()` calls don't accumulate
// elements (which trips `getByTestId`'s multiple-elements error). Also
// imports jest-dom matchers globally so individual test files don't
// have to repeat the import.

import '@testing-library/jest-dom/vitest';
import { afterEach } from 'vitest';
import { cleanup } from '@testing-library/react';

afterEach(() => {
  cleanup();
});
