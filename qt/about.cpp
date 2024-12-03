#include "about_dialog_ui.h"
#include "platform/platform.hpp"

#include "base/logging.hpp"

#include <sstream>
#include <fstream>

class AboutDialog {
public:
    explicit AboutDialog() {
        m_ui = AboutDialogUI::create();

        auto &platform = GetPlatform();
        m_ui->set_app_name(QCoreApplication::applicationName().toStdString());
        m_ui->set_version(platform.Version());

        std::string aboutText;
        try {
            ReaderPtr<Reader> reader = platform.GetReader("copyright.html");
            reader.ReadAsString(aboutText);
        } catch (RootException const &ex) {
            LOG(LWARNING, ("About text error: ", ex.Msg()));
            aboutText = "Error loading about text.";
        }
        m_ui->set_about_text(aboutText);

        m_ui->on_close([&]() { close(); });
    }

    void show() {
        m_ui->show();
    }

    void close() {
        m_ui->hide();
    }

private:
    std::shared_ptr<AboutDialogUI> m_ui;
};
