import { Component, JSX } from 'solid-js';

const Heading3: Component<JSX.HTMLAttributes<HTMLParagraphElement>> = (props) => {
  return (
    <p {...props} class={`archivo-black w-fit text-sm tracking-tighter ${props.class}`}>
      {props.children}
    </p>
  );
};

export default Heading3;
