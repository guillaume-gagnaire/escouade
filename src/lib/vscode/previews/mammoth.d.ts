// mammoth's prebuilt browser bundle (no Node module in it), typed as its package.
declare module 'mammoth/mammoth.browser.min.js' {
  const mammoth: typeof import('mammoth');
  export default mammoth;
}
