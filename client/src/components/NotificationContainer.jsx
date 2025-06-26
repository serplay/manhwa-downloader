export default function NotificationContainer({ children }) {
  return (
    <div className="fixed top-4 left-1/2 transform -translate-x-1/2 z-50 flex flex-col items-center gap-3 w-full px-4 max-w-md pointer-events-none">
      {children}
    </div>
  );
}
