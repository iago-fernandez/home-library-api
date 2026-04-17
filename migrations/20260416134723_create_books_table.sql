CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE IF NOT EXISTS books (
    -- Internal tracking and identification
     id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
     catalog_number SERIAL UNIQUE,
     isbn_13 VARCHAR(13) UNIQUE,
     isbn_10 VARCHAR(10),
     open_library_id VARCHAR(50),
     oclc_number VARCHAR(50),
    
    -- Primary bibliographic data
     title VARCHAR(255) NOT NULL,
     subtitle VARCHAR(255),
     original_title VARCHAR(255),
     authors TEXT[] NOT NULL DEFAULT '{}',
     translators TEXT[] DEFAULT '{}',
     illustrators TEXT[] DEFAULT '{}',

    -- Edition and publication details
     publisher VARCHAR(255),
     publish_date VARCHAR(50),
     original_publish_date VARCHAR(50),
     edition_number VARCHAR(50),
     printing_number VARCHAR(50),
     original_edition VARCHAR(255),
     is_first_edition BOOLEAN DEFAULT FALSE,
     collection_name VARCHAR(255),
     volume_in_collection INTEGER,
     series_name VARCHAR(255),
     volume_in_series INTEGER,

    -- Physical and structural properties
     book_format VARCHAR(100),
     page_count INTEGER,
     dimensions VARCHAR(100),
     weight VARCHAR(50),
     language VARCHAR(50),
     original_language VARCHAR(50),

    -- Content classification
     subjects TEXT[] DEFAULT '{}',
     genres TEXT[] DEFAULT '{}',
     target_audience VARCHAR(100),
     description TEXT,
     table_of_contents TEXT,
     cover_url VARCHAR(512),

    -- Acquisition and logistics
     purchase_date VARCHAR(50),
     purchase_price DECIMAL(10, 2),
     store_or_vendor VARCHAR(255),
     acquisition_type VARCHAR(100),

    -- Physical location within premises
     location_property VARCHAR(100),
     location_room VARCHAR(100),
     location_bookcase VARCHAR(100),
     location_shelf VARCHAR(100),
     location_position INTEGER,

    -- Condition and personal notes
     condition_state VARCHAR(100),
     personal_notes TEXT,

    -- Reading progress and personal assessment
     read_status VARCHAR(50) DEFAULT 'unread',
     rating INTEGER CHECK (rating >= 0 AND rating <= 10),
     date_started VARCHAR(50),
     date_finished VARCHAR(50),
     reading_notes TEXT,

    -- Loan management
     is_loaned BOOLEAN DEFAULT FALSE,
     loaned_to VARCHAR(255),
     loan_date TIMESTAMPTZ,
     expected_return_date TIMESTAMPTZ,

    -- System auditing
     created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
     updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);