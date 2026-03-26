import { Component, JSX } from 'solid-js';

const Title: Component<JSX.HTMLAttributes<HTMLParagraphElement>> = (props) => {
  return (
    <p class='archivo-black text-zinc-700 text-xl italic font-extrabold tracking-tighter scale-x-150 origin-top-left' {...props}>
      {props.children}
    </p>
  );
};

export default Title;
