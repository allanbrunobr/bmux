import type { Metadata } from 'next'
import './globals.css'
import { Sidebar } from '@/components/layout/Sidebar'
import { Header } from '@/components/layout/Header'
import { BmuxProvider } from '@/components/BmuxProvider'
import { AdversarialPanelWrapper } from '@/components/AdversarialPanelWrapper'
import { SendTaskModal } from '@/components/SendTaskModal'
import { Toaster } from '@/components/ui/toaster'

export const metadata: Metadata = {
  title: 'BMUX Dashboard',
  description: 'Real-time dashboard for BMUX AI agent sessions',
}

export default function RootLayout({
  children,
}: {
  children: React.ReactNode
}) {
  return (
    <html lang="en" className="dark">
      <body className="min-h-screen bg-background text-foreground antialiased">
        <BmuxProvider>
          <div className="flex h-screen overflow-hidden">
            <Sidebar />
            <div className="flex flex-col flex-1 min-w-0 overflow-hidden">
              <Header />
              <AdversarialPanelWrapper />
              <main className="flex-1 overflow-y-auto p-6">
                {children}
              </main>
            </div>
          </div>
          <SendTaskModal />
          <Toaster />
        </BmuxProvider>
      </body>
    </html>
  )
}
