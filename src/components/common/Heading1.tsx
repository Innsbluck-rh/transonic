import { Component, JSX } from 'solid-js';

const Heading1: Component<JSX.HTMLAttributes<HTMLParagraphElement>> = (props) => {
  return (
    <p class='archivo-black origin-top-left scale-x-150 text-lg font-extrabold italic tracking-tighter text-zinc-700' {...props}>
      {props.children}
    </p>
  );
};

export default Heading1;
