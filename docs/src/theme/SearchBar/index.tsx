import React from 'react';
import SearchBar from '@theme-original/SearchBar';
import type SearchBarType from '@theme/SearchBar';
import type {WrapperProps} from '@docusaurus/types';

declare global {
  interface Window {
    runllm?: {
      open: () => Promise<void>;
      close: () => Promise<void>;
      sendMessage: (message: string) => Promise<void>;
    };
  }
}

type Props = WrapperProps<typeof SearchBarType>;

export default function SearchBarWrapper(props: Props): JSX.Element {
  console.log('props', props); // Figure out what other props to use
  return (
    <>
      <button onClick={() => {
        if (!!window.runllm) {
          window.runllm.open();
        }
      }}>
        Search / AI Assist
      </button>
    </>
  );
}