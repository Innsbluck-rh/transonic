import { Component, JSX } from 'solid-js';

const Heading2: Component<JSX.HTMLAttributes<HTMLParagraphElement>> = (props) => {
  return (
    <p class='archivo-black text-xs origin-top-left scale-x-150 font-extrabold italic tracking-tighter text-zinc-700' {...props}>
      {props.children}
    </p>
  );
};

export default Heading2;
